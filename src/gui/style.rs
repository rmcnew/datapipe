//! Material Design 3 (M3) visual theme, tonal color tokens, typography, and reusable components.
//!
//! Implements the [Material Design 3](https://m3.material.io/) specification with a Tonal Blue
//! color scheme in both Light and Dark modes, full surface-container elevation hierarchy,
//! stadium-pill action buttons with vector-painted icons and hover/press state layers,
//! M3 Primary Tabs, animated M3 Switches, Outlined/Elevated Cards, and semantic status chips.

use crate::datapipe_types::EncryptionKey;
use egui::{
    Color32, CornerRadius, CursorIcon, Event, FontFamily, FontId, Frame, Id, Key, Margin, Painter,
    PointerButton, Popup, Pos2, ProgressBar, Rect, Response, RichText, Sense, Shadow, Shape,
    Stroke, StrokeKind, Style, TextBuffer as _, TextStyle, Theme, Ui, Vec2, ViewportCommand,
    Visuals, WidgetInfo, pos2,
    style::Selection,
    text::{CCursor, CCursorRange},
    text_edit::TextEditState,
    vec2,
};

/// Material Design 3 color scheme mode (`Light` or `Dark`).
///
/// # Examples
/// ```
/// use datapipe::gui::style::ThemeMode;
///
/// let mut mode = ThemeMode::default();
/// assert_eq!(mode, ThemeMode::Light);
/// mode.toggle();
/// assert_eq!(mode, ThemeMode::Dark);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    /// Daylight-optimized Material Design 3 tonal scheme (default).
    #[default]
    Light,
    /// Low-luminance Material Design 3 dark tonal scheme.
    Dark,
}

impl ThemeMode {
    /// Returns `true` if this mode is [`ThemeMode::Dark`].
    #[must_use]
    pub const fn is_dark(self) -> bool {
        matches!(self, Self::Dark)
    }

    /// Toggles between [`ThemeMode::Light`] and [`ThemeMode::Dark`].
    pub fn toggle(&mut self) {
        *self = match self {
            Self::Light => Self::Dark,
            Self::Dark => Self::Light,
        };
    }

    /// Converts this [`ThemeMode`] into the corresponding [`egui::Theme`].
    #[must_use]
    pub const fn to_egui_theme(self) -> Theme {
        match self {
            Self::Light => Theme::Light,
            Self::Dark => Theme::Dark,
        }
    }
}

impl std::fmt::Display for ThemeMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Light => f.write_str("Light"),
            Self::Dark => f.write_str("Dark"),
        }
    }
}

/// Complete set of Material Design 3 semantic color tokens for a single [`ThemeMode`].
///
/// # Examples
/// ```
/// use datapipe::gui::style::{M3Palette, ThemeMode};
///
/// let light = M3Palette::for_mode(ThemeMode::Light);
/// let dark = M3Palette::for_mode(ThemeMode::Dark);
/// assert_ne!(light.surface, dark.surface);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct M3Palette {
    /// High-emphasis brand and active indicator fill (`#0B57D0` Light / `#A8C7FA` Dark).
    pub primary: Color32,
    /// Text and icons placed on top of [`Self::primary`].
    pub on_primary: Color32,
    /// Standout tonal container fill paired with [`Self::primary`].
    pub primary_container: Color32,
    /// Text and icons placed on top of [`Self::primary_container`].
    pub on_primary_container: Color32,

    /// Supporting accent color for medium-emphasis components.
    pub secondary: Color32,
    /// Text and icons placed on top of [`Self::secondary`].
    pub on_secondary: Color32,
    /// Tonal fill used by M3 Filled Tonal buttons and secondary badges.
    pub secondary_container: Color32,
    /// Text and icons placed on top of [`Self::secondary_container`].
    pub on_secondary_container: Color32,

    /// Complementary accent used for positive/active pipeline indicators and Start controls.
    pub tertiary: Color32,
    /// Text and icons placed on top of [`Self::tertiary`].
    pub on_tertiary: Color32,
    /// Tonal container fill for valid/running states.
    pub tertiary_container: Color32,
    /// Text and icons placed on top of [`Self::tertiary_container`].
    pub on_tertiary_container: Color32,

    /// High-emphasis destructive / invalid state color.
    pub error: Color32,
    /// Text and icons placed on top of [`Self::error`].
    pub on_error: Color32,
    /// Tonal container fill for validation errors and alerts.
    pub error_container: Color32,
    /// Text and icons placed on top of [`Self::error_container`].
    pub on_error_container: Color32,

    /// Caution accent color for insecure TLS options and stopped pipeline states.
    pub warning: Color32,
    /// Tonal container fill for caution banners and chips.
    pub warning_container: Color32,
    /// Text and icons placed on top of [`Self::warning_container`].
    pub on_warning_container: Color32,

    /// Primary window canvas background.
    pub surface: Color32,
    /// High-emphasis primary text color on [`Self::surface`].
    pub on_surface: Color32,
    /// Medium-emphasis supporting text and inactive tab label color.
    pub on_surface_variant: Color32,

    /// Elevation 0 container fill (e.g., text fields and crisp inner surfaces).
    pub surface_container_lowest: Color32,
    /// Elevation 1 container fill (e.g., M3 Cards, Top App Bar, Bottom Action Bar).
    pub surface_container_low: Color32,
    /// Elevation 2 container fill (e.g., menus, popups, inner diagram canvas).
    pub surface_container: Color32,
    /// Elevation 3 container fill (e.g., metric tiles and hover cards).
    pub surface_container_high: Color32,
    /// Elevation 4 container fill (e.g., unselected switch tracks and progress tracks).
    pub surface_container_highest: Color32,

    /// High-contrast border color for text inputs, outlined buttons, and switches.
    pub outline: Color32,
    /// Low-contrast structural divider and card border color.
    pub outline_variant: Color32,
}

impl M3Palette {
    /// Material Design 3 Tonal Blue Light color scheme.
    #[must_use]
    pub const fn light() -> Self {
        Self {
            primary: Color32::from_rgb(11, 87, 208),
            on_primary: Color32::from_rgb(255, 255, 255),
            primary_container: Color32::from_rgb(211, 227, 253),
            on_primary_container: Color32::from_rgb(4, 30, 73),

            secondary: Color32::from_rgb(68, 94, 145),
            on_secondary: Color32::from_rgb(255, 255, 255),
            secondary_container: Color32::from_rgb(216, 226, 255),
            on_secondary_container: Color32::from_rgb(0, 26, 65),

            tertiary: Color32::from_rgb(20, 108, 46),
            on_tertiary: Color32::from_rgb(255, 255, 255),
            tertiary_container: Color32::from_rgb(196, 238, 208),
            on_tertiary_container: Color32::from_rgb(7, 39, 17),

            error: Color32::from_rgb(179, 38, 30),
            on_error: Color32::from_rgb(255, 255, 255),
            error_container: Color32::from_rgb(249, 222, 220),
            on_error_container: Color32::from_rgb(65, 14, 11),

            warning: Color32::from_rgb(176, 96, 0),
            warning_container: Color32::from_rgb(255, 239, 213),
            on_warning_container: Color32::from_rgb(62, 29, 0),

            surface: Color32::from_rgb(248, 250, 253),
            on_surface: Color32::from_rgb(25, 28, 32),
            on_surface_variant: Color32::from_rgb(68, 71, 78),

            surface_container_lowest: Color32::from_rgb(255, 255, 255),
            surface_container_low: Color32::from_rgb(242, 245, 250),
            surface_container: Color32::from_rgb(236, 239, 246),
            surface_container_high: Color32::from_rgb(230, 233, 240),
            surface_container_highest: Color32::from_rgb(224, 227, 235),

            outline: Color32::from_rgb(116, 119, 127),
            outline_variant: Color32::from_rgb(196, 198, 208),
        }
    }

    /// Material Design 3 Tonal Blue Dark color scheme.
    #[must_use]
    pub const fn dark() -> Self {
        Self {
            primary: Color32::from_rgb(168, 199, 250),
            on_primary: Color32::from_rgb(6, 46, 111),
            primary_container: Color32::from_rgb(8, 66, 160),
            on_primary_container: Color32::from_rgb(211, 227, 253),

            secondary: Color32::from_rgb(174, 198, 255),
            on_secondary: Color32::from_rgb(17, 47, 96),
            secondary_container: Color32::from_rgb(43, 70, 120),
            on_secondary_container: Color32::from_rgb(216, 226, 255),

            tertiary: Color32::from_rgb(109, 213, 140),
            on_tertiary: Color32::from_rgb(10, 56, 24),
            tertiary_container: Color32::from_rgb(15, 82, 35),
            on_tertiary_container: Color32::from_rgb(196, 238, 208),

            error: Color32::from_rgb(242, 184, 181),
            on_error: Color32::from_rgb(96, 20, 16),
            error_container: Color32::from_rgb(140, 29, 24),
            on_error_container: Color32::from_rgb(249, 222, 220),

            warning: Color32::from_rgb(255, 185, 81),
            warning_container: Color32::from_rgb(92, 56, 0),
            on_warning_container: Color32::from_rgb(255, 221, 179),

            surface: Color32::from_rgb(17, 19, 24),
            on_surface: Color32::from_rgb(226, 226, 233),
            on_surface_variant: Color32::from_rgb(196, 198, 208),

            surface_container_lowest: Color32::from_rgb(12, 14, 19),
            surface_container_low: Color32::from_rgb(25, 28, 32),
            surface_container: Color32::from_rgb(29, 32, 36),
            surface_container_high: Color32::from_rgb(40, 42, 47),
            surface_container_highest: Color32::from_rgb(51, 53, 58),

            outline: Color32::from_rgb(142, 144, 153),
            outline_variant: Color32::from_rgb(68, 71, 78),
        }
    }

    /// Returns the [`M3Palette`] matching the requested [`ThemeMode`].
    #[must_use]
    pub const fn for_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Light => Self::light(),
            ThemeMode::Dark => Self::dark(),
        }
    }

    /// Blends an M3 state-layer overlay color onto an opaque base color.
    #[must_use]
    pub fn state_layer(base: Color32, overlay: Color32, alpha: f32) -> Color32 {
        let t = alpha.clamp(0.0, 1.0);
        let inv = 1.0 - t;
        let r = (f32::from(base.r()) * inv + f32::from(overlay.r()) * t).round() as u8;
        let g = (f32::from(base.g()) * inv + f32::from(overlay.g()) * t).round() as u8;
        let b = (f32::from(base.b()) * inv + f32::from(overlay.b()) * t).round() as u8;
        Color32::from_rgb(r, g, b)
    }

    /// M3 disabled container fill (`on_surface` at 12% blended over `surface`).
    #[must_use]
    pub fn disabled_container(self) -> Color32 {
        Self::state_layer(self.surface, self.on_surface, 0.12)
    }

    /// M3 disabled content / foreground color (`on_surface` at 38% blended over `surface`).
    #[must_use]
    pub fn disabled_content(self) -> Color32 {
        Self::state_layer(self.surface, self.on_surface, 0.38)
    }
}

/// Default Light-mode window surface constant retained for backwards compatibility.
pub const WINDOW_BG: Color32 = M3Palette::light().surface;
/// Default Light-mode card container fill constant retained for backwards compatibility.
pub const PANEL_BG: Color32 = M3Palette::light().surface_container_low;
/// Default Light-mode primary color constant retained for backwards compatibility.
pub const TAB_ACTIVE_BG: Color32 = M3Palette::light().primary_container;
/// Default Light-mode inactive tab container constant retained for backwards compatibility.
pub const TAB_INACTIVE_BG: Color32 = M3Palette::light().surface_container_low;
/// Default Light-mode outline border constant retained for backwards compatibility.
pub const BORDER_COLOR: Color32 = M3Palette::light().outline_variant;
/// Default Light-mode primary text color constant retained for backwards compatibility.
pub const TEXT_DARK: Color32 = M3Palette::light().on_surface;
/// Default Light-mode tonal button fill constant retained for backwards compatibility.
pub const BLUE_BUTTON_BG: Color32 = M3Palette::light().secondary_container;
/// Default Light-mode destructive button fill constant retained for backwards compatibility.
pub const STOP_BUTTON_BG: Color32 = M3Palette::light().error;
/// Default Light-mode positive action button fill constant retained for backwards compatibility.
pub const START_BUTTON_BG: Color32 = M3Palette::light().tertiary;
/// Default Light-mode disabled button fill constant retained for backwards compatibility.
pub const DISABLED_BG: Color32 = Color32::from_rgb(221, 224, 228);

/// Returns the active [`M3Palette`] for the given [`Ui`] based on `ui.visuals().dark_mode`.
#[must_use]
pub fn palette(ui: &Ui) -> M3Palette {
    if ui.visuals().dark_mode {
        M3Palette::dark()
    } else {
        M3Palette::light()
    }
}

/// Builds a complete [`egui::Style`] implementing Material Design 3 typography, spacing,
/// corner radii, and tonal surface visuals for `mode`.
#[must_use]
pub fn build_m3_style(mode: ThemeMode) -> Style {
    let p = M3Palette::for_mode(mode);

    // Material Design 3 Typography Scale
    let mut style = Style {
        text_styles: [
            (
                TextStyle::Heading,
                FontId::new(20.0, FontFamily::Proportional),
            ),
            (TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
            (
                TextStyle::Button,
                FontId::new(14.0, FontFamily::Proportional),
            ),
            (
                TextStyle::Monospace,
                FontId::new(13.5, FontFamily::Monospace),
            ),
            (
                TextStyle::Small,
                FontId::new(12.0, FontFamily::Proportional),
            ),
        ]
        .into(),
        ..Default::default()
    };

    // Material Design 3 Spacing & Density
    style.spacing.item_spacing = vec2(10.0, 8.0);
    style.spacing.button_padding = vec2(14.0, 7.0);
    style.spacing.interact_size = vec2(44.0, 32.0);
    style.spacing.window_margin = Margin::same(14);
    style.spacing.menu_margin = Margin::same(8);
    style.spacing.icon_width = 18.0;
    style.spacing.icon_width_inner = 10.0;
    style.spacing.icon_spacing = 8.0;
    style.spacing.combo_height = 240.0;

    let mut visuals = if mode.is_dark() {
        Visuals::dark()
    } else {
        Visuals::light()
    };

    visuals.panel_fill = p.surface;
    visuals.window_fill = p.surface_container;
    visuals.window_stroke = Stroke::new(1.0, p.outline_variant);
    visuals.window_corner_radius = CornerRadius::same(12);
    visuals.menu_corner_radius = CornerRadius::same(12);
    visuals.extreme_bg_color = p.surface_container_lowest;
    visuals.text_edit_bg_color = Some(p.surface_container_lowest);
    visuals.faint_bg_color = p.surface_container_low;
    visuals.code_bg_color = p.surface_container_high;
    visuals.warn_fg_color = p.warning;
    visuals.error_fg_color = p.error;
    visuals.hyperlink_color = p.primary;
    visuals.interact_cursor = Some(CursorIcon::PointingHand);
    visuals.selection = Selection {
        bg_fill: p.primary_container,
        stroke: Stroke::new(1.0, p.on_primary_container),
    };
    visuals.text_cursor.stroke = Stroke::new(2.0, p.primary);

    let shadow_alpha = if mode.is_dark() { 90 } else { 28 };
    visuals.popup_shadow = Shadow {
        offset: [0, 4],
        blur: 12,
        spread: 0,
        color: Color32::from_black_alpha(shadow_alpha),
    };
    visuals.window_shadow = Shadow {
        offset: [0, 6],
        blur: 16,
        spread: 0,
        color: Color32::from_black_alpha(shadow_alpha),
    };

    // Non-interactive containers & text
    visuals.widgets.noninteractive.bg_fill = p.surface_container_low;
    visuals.widgets.noninteractive.weak_bg_fill = p.surface_container_low;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.outline_variant);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, p.on_surface);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(8);

    // Resting interactive widgets (M3 Outlined TextFields / Checkboxes + Tonal default buttons)
    visuals.widgets.inactive.bg_fill = p.surface_container_lowest;
    visuals.widgets.inactive.weak_bg_fill = p.secondary_container;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, p.outline);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.2, p.on_surface);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(8);
    visuals.widgets.inactive.expansion = 0.0;

    // Hovered state (8% state layer + 1.5dp primary focus/hover border)
    visuals.widgets.hovered.bg_fill = p.surface_container_low;
    visuals.widgets.hovered.weak_bg_fill =
        M3Palette::state_layer(p.secondary_container, p.on_secondary_container, 0.10);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.5, p.primary);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.5, p.on_surface);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(8);
    visuals.widgets.hovered.expansion = 0.0;

    // Pressed / focused state (12% state layer + 2dp primary active border)
    visuals.widgets.active.bg_fill = p.surface_container_lowest;
    visuals.widgets.active.weak_bg_fill =
        M3Palette::state_layer(p.secondary_container, p.on_secondary_container, 0.16);
    visuals.widgets.active.bg_stroke = Stroke::new(2.0, p.primary);
    visuals.widgets.active.fg_stroke = Stroke::new(1.5, p.primary);
    visuals.widgets.active.corner_radius = CornerRadius::same(8);
    visuals.widgets.active.expansion = 0.0;

    // Open menu / ComboBox state
    visuals.widgets.open.bg_fill = p.primary_container;
    visuals.widgets.open.weak_bg_fill = p.primary_container;
    visuals.widgets.open.bg_stroke = Stroke::new(1.5, p.primary);
    visuals.widgets.open.fg_stroke = Stroke::new(1.5, p.on_primary_container);
    visuals.widgets.open.corner_radius = CornerRadius::same(8);
    visuals.widgets.open.expansion = 0.0;

    style.visuals = visuals;
    style
}

/// Installs both Light and Dark Material Design 3 styles on `ctx` and activates `mode`.
///
/// # Examples
/// ```
/// use datapipe::gui::style::{ThemeMode, apply_theme};
///
/// let ctx = egui::Context::default();
/// apply_theme(&ctx, ThemeMode::Dark);
/// ```
pub fn apply_theme(ctx: &egui::Context, mode: ThemeMode) {
    ctx.set_style_of(Theme::Light, build_m3_style(ThemeMode::Light));
    ctx.set_style_of(Theme::Dark, build_m3_style(ThemeMode::Dark));
    ctx.set_theme(mode.to_egui_theme());
}

/// Configures the [`egui::Context`] with the default Material Design 3 Light theme.
pub fn configure_visuals(ctx: &egui::Context) {
    apply_theme(ctx, ThemeMode::Light);
}

/// Returns an M3 Card [`Frame`] (`12dp` corner radius, `surface_container_low` fill,
/// `1dp` `outline_variant` border, and `16dp` inner padding).
pub fn m3_card(ui: &Ui) -> Frame {
    let p = palette(ui);
    Frame::NONE
        .fill(p.surface_container_low)
        .stroke(Stroke::new(1.0, p.outline_variant))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::same(16))
}

/// Returns a higher-elevation M3 Tonal Card [`Frame`] (`surface_container` fill, `12dp` radius)
/// suitable for nested metric tiles and summary callouts.
pub fn m3_tonal_card(ui: &Ui) -> Frame {
    let p = palette(ui);
    Frame::NONE
        .fill(p.surface_container)
        .stroke(Stroke::new(1.0, p.outline_variant))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::same(14))
}

/// Renders an M3 section header with a `TitleMedium` heading and `BodySmall` supporting text.
pub fn section_header(ui: &mut Ui, title: &str, subtitle: &str) {
    let p = palette(ui);
    ui.label(RichText::new(title).size(18.0).strong().color(p.on_surface));
    if !subtitle.is_empty() {
        ui.add_space(2.0);
        ui.label(
            RichText::new(subtitle)
                .size(12.5)
                .color(p.on_surface_variant),
        );
    }
}

/// Renders an M3 pill-shaped tonal status or validation chip (`8dp` corner radius).
pub fn status_chip(ui: &mut Ui, label: &str, container: Color32, on_container: Color32) {
    status_chip_with_dot(ui, label, container, on_container, false);
}

/// Renders an M3 pill-shaped tonal status chip with an optional vector-painted leading dot.
pub fn status_chip_with_dot(
    ui: &mut Ui,
    label: &str,
    container: Color32,
    on_container: Color32,
    show_dot: bool,
) {
    Frame::NONE
        .fill(container)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(10, 4))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                if show_dot {
                    let (dot_rect, _) = ui.allocate_exact_size(vec2(8.0, 8.0), Sense::hover());
                    ui.painter()
                        .circle_filled(dot_rect.center(), 3.5, on_container);
                }
                ui.label(RichText::new(label).size(12.0).strong().color(on_container));
            });
        });
}

/// Paints the bidirectional data-stream brand emblem inside `rect` using crisp vector strokes.
pub fn paint_brand_icon(painter: &Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let half_w = 7.5;
    let tip = 3.6;
    let stroke = Stroke::new(2.0, color);

    // Top arrow pointing right
    let y_top = c.y - 4.0;
    let r_start = pos2(c.x - half_w, y_top);
    let r_end = pos2(c.x + half_w, y_top);
    painter.line_segment([r_start, r_end], stroke);
    painter.line_segment([r_end, pos2(r_end.x - tip, y_top - tip)], stroke);
    painter.line_segment([r_end, pos2(r_end.x - tip, y_top + tip)], stroke);

    // Bottom arrow pointing left
    let y_bot = c.y + 4.0;
    let l_start = pos2(c.x + half_w, y_bot);
    let l_end = pos2(c.x - half_w, y_bot);
    painter.line_segment([l_start, l_end], stroke);
    painter.line_segment([l_end, pos2(l_end.x + tip, y_bot - tip)], stroke);
    painter.line_segment([l_end, pos2(l_end.x + tip, y_bot + tip)], stroke);
}

/// Vector icons painted directly via [`egui::Painter`] so buttons never depend on emoji/symbol
/// glyphs that may be absent from bundled text fonts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum M3Icon {
    ArrowLeft,
    ArrowRight,
    Play,
    Stop,
    Plus,
    Sun,
    Moon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IconPlacement {
    None,
    Leading(M3Icon),
    Trailing(M3Icon),
}

fn paint_m3_icon(painter: &Painter, icon: M3Icon, rect: Rect, fg: Color32, bg_behind: Color32) {
    let c = rect.center();
    let stroke = Stroke::new(1.8, fg);

    match icon {
        M3Icon::ArrowLeft => {
            let left = pos2(c.x - 5.5, c.y);
            let right = pos2(c.x + 5.5, c.y);
            let tip = 4.2;
            painter.line_segment([left, right], stroke);
            painter.line_segment([left, pos2(left.x + tip, left.y - tip)], stroke);
            painter.line_segment([left, pos2(left.x + tip, left.y + tip)], stroke);
        }
        M3Icon::ArrowRight => {
            let left = pos2(c.x - 5.5, c.y);
            let right = pos2(c.x + 5.5, c.y);
            let tip = 4.2;
            painter.line_segment([left, right], stroke);
            painter.line_segment([right, pos2(right.x - tip, right.y - tip)], stroke);
            painter.line_segment([right, pos2(right.x - tip, right.y + tip)], stroke);
        }
        M3Icon::Play => {
            let pts = vec![
                pos2(c.x - 4.0, c.y - 5.5),
                pos2(c.x + 5.5, c.y),
                pos2(c.x - 4.0, c.y + 5.5),
            ];
            painter.add(Shape::convex_polygon(pts, fg, Stroke::NONE));
        }
        M3Icon::Stop => {
            let sq = Rect::from_center_size(c, vec2(9.6, 9.6));
            painter.rect_filled(sq, CornerRadius::same(2), fg);
        }
        M3Icon::Plus => {
            let half = 5.0;
            painter.line_segment([pos2(c.x - half, c.y), pos2(c.x + half, c.y)], stroke);
            painter.line_segment([pos2(c.x, c.y - half), pos2(c.x, c.y + half)], stroke);
        }
        M3Icon::Sun => {
            painter.circle_stroke(c, 3.2, Stroke::new(1.6, fg));
            let r_in = 5.0_f32;
            let r_out = 7.0_f32;
            for step in 0..8 {
                let angle = (step as f32) * core::f32::consts::FRAC_PI_4;
                let (sin, cos) = angle.sin_cos();
                painter.line_segment(
                    [
                        pos2(c.x + cos * r_in, c.y + sin * r_in),
                        pos2(c.x + cos * r_out, c.y + sin * r_out),
                    ],
                    Stroke::new(1.5, fg),
                );
            }
        }
        M3Icon::Moon => {
            painter.circle_filled(c, 5.6, fg);
            painter.circle_filled(pos2(c.x + 2.4, c.y - 1.8), 4.6, bg_behind);
        }
    }
}

/// Renders an M3 Primary Tab (`48dp` height) with hover state layer, optional tonal badge,
/// and a `3dp` rounded active indicator bar along the bottom edge.
pub fn primary_tab(
    ui: &mut Ui,
    title: &str,
    badge: Option<&str>,
    is_selected: bool,
    width: f32,
) -> Response {
    let p = palette(ui);
    let desired_size = vec2(width.max(48.0), 48.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());

    response.widget_info(|| {
        WidgetInfo::selected(egui::Role::Button, ui.is_enabled(), is_selected, title)
    });

    if response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();

        // State layer on hover / press
        if response.is_pointer_button_down_on() {
            painter.rect_filled(rect, CornerRadius::ZERO, p.primary.gamma_multiply(0.12));
        } else if response.hovered() {
            painter.rect_filled(rect, CornerRadius::ZERO, p.primary.gamma_multiply(0.08));
        }

        let label_color = if is_selected {
            p.primary
        } else if response.hovered() {
            p.on_surface
        } else {
            p.on_surface_variant
        };

        let font_id = FontId::new(14.0, FontFamily::Proportional);
        let title_galley = painter.layout_no_wrap(title.to_string(), font_id, label_color);

        let badge_galley = badge.map(|b| {
            let badge_fg = if is_selected {
                p.on_primary_container
            } else {
                p.on_secondary_container
            };
            painter.layout_no_wrap(
                b.to_string(),
                FontId::new(11.0, FontFamily::Proportional),
                badge_fg,
            )
        });

        let badge_pad_x = 6.0;
        let badge_gap = 6.0;
        let badge_total_w = badge_galley
            .as_ref()
            .map_or(0.0, |bg| bg.size().x + badge_pad_x * 2.0 + badge_gap);
        let content_width = title_galley.size().x + badge_total_w;

        let start_x = rect.center().x - content_width * 0.5;
        let title_pos = pos2(start_x, rect.center().y - title_galley.size().y * 0.5);
        painter.galley(title_pos, title_galley.clone(), label_color);

        if let Some(bg) = badge_galley {
            let pill_w = bg.size().x + badge_pad_x * 2.0;
            let pill_h = 18.0;
            let pill_x = start_x + title_galley.size().x + badge_gap;
            let pill_rect = Rect::from_min_size(
                pos2(pill_x, rect.center().y - pill_h * 0.5),
                vec2(pill_w, pill_h),
            );
            let pill_fill = if is_selected {
                p.primary_container
            } else {
                p.secondary_container
            };
            let pill_fg = if is_selected {
                p.on_primary_container
            } else {
                p.on_secondary_container
            };
            painter.rect_filled(pill_rect, CornerRadius::same(9), pill_fill);
            painter.galley(
                pos2(
                    pill_rect.center().x - bg.size().x * 0.5,
                    pill_rect.center().y - bg.size().y * 0.5,
                ),
                bg,
                pill_fg,
            );
        }

        // M3 3dp pill-top active indicator bar
        if is_selected {
            let indicator_w = (content_width + 8.0).clamp(36.0, (rect.width() - 16.0).max(36.0));
            let indicator_rect = Rect::from_min_size(
                pos2(rect.center().x - indicator_w * 0.5, rect.max.y - 3.0),
                vec2(indicator_w, 3.0),
            );
            painter.rect_filled(
                indicator_rect,
                CornerRadius {
                    nw: 3,
                    ne: 3,
                    sw: 0,
                    se: 0,
                },
                p.primary,
            );
        }
    }

    response
}

/// Renders a top-level M3 Primary Tab without a badge.
pub fn tab_button(ui: &mut Ui, title: &str, is_selected: bool, width: f32) -> Response {
    primary_tab(ui, title, None, is_selected, width)
}

/// Internal helper to render an M3 stadium-pill button (`CornerRadius::same(height / 2)`) with
/// optional vector-painted leading/trailing icon, hover (8%) and press (14%) state layers,
/// and M3 disabled container/content tokens.
#[expect(clippy::too_many_arguments)]
fn render_pill_button_with_icon(
    ui: &mut Ui,
    text: &str,
    icon_placement: IconPlacement,
    enabled: bool,
    min_size: Vec2,
    fill: Color32,
    fg: Color32,
    border: Option<Stroke>,
) -> Response {
    let p = palette(ui);
    let is_enabled = enabled && ui.is_enabled();

    let effective_fg = if is_enabled { fg } else { p.disabled_content() };

    let font_id = FontId::new(14.0, FontFamily::Proportional);
    let galley = if text.is_empty() {
        None
    } else {
        Some(
            ui.painter()
                .layout_no_wrap(text.to_string(), font_id, effective_fg),
        )
    };

    let icon_box = 14.0;
    let has_icon = !matches!(icon_placement, IconPlacement::None);
    let text_w = galley.as_ref().map_or(0.0, |g| g.size().x);
    let icon_gap = if has_icon && text_w > 0.0 { 8.0 } else { 0.0 };
    let content_w = text_w + if has_icon { icon_box } else { 0.0 } + icon_gap;

    let horiz_pad = if has_icon { 34.0 } else { 32.0 };
    let desired_size = vec2(
        (content_w + horiz_pad).max(min_size.x),
        min_size.y.max(32.0),
    );

    let sense = if is_enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(desired_size, sense);

    response.widget_info(|| WidgetInfo::labeled(egui::Role::Button, is_enabled, text));

    if is_enabled && response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let pill_radius = (rect.height() * 0.5).round() as u8;
        let pill = CornerRadius::same(pill_radius);

        let hover_base = if fill == Color32::TRANSPARENT {
            p.surface_container_low
        } else {
            fill
        };

        let (effective_fill, effective_stroke) = if is_enabled {
            let state_fill = if response.is_pointer_button_down_on() {
                M3Palette::state_layer(hover_base, fg, 0.14)
            } else if response.hovered() {
                M3Palette::state_layer(hover_base, fg, 0.08)
            } else {
                fill
            };
            (state_fill, border.unwrap_or(Stroke::NONE))
        } else if border.is_some() {
            (Color32::TRANSPARENT, Stroke::new(1.0, p.outline_variant))
        } else {
            (p.disabled_container(), Stroke::NONE)
        };

        if effective_fill != Color32::TRANSPARENT {
            painter.rect_filled(rect, pill, effective_fill);
        }
        if effective_stroke != Stroke::NONE {
            painter.rect_stroke(rect, pill, effective_stroke, StrokeKind::Inside);
        }

        let bg_behind_icon = if effective_fill == Color32::TRANSPARENT {
            hover_base
        } else {
            effective_fill
        };

        let start_x = rect.center().x - content_w * 0.5;
        let center_y = rect.center().y;

        match icon_placement {
            IconPlacement::None => {
                if let Some(g) = galley {
                    painter.galley(
                        pos2(
                            rect.center().x - g.size().x * 0.5,
                            center_y - g.size().y * 0.5,
                        ),
                        g,
                        effective_fg,
                    );
                }
            }
            IconPlacement::Leading(icon) => {
                let icon_rect = Rect::from_min_size(
                    pos2(start_x, center_y - icon_box * 0.5),
                    vec2(icon_box, icon_box),
                );
                paint_m3_icon(painter, icon, icon_rect, effective_fg, bg_behind_icon);
                if let Some(g) = galley {
                    painter.galley(
                        pos2(start_x + icon_box + icon_gap, center_y - g.size().y * 0.5),
                        g,
                        effective_fg,
                    );
                }
            }
            IconPlacement::Trailing(icon) => {
                let mut cursor_x = start_x;
                if let Some(g) = galley {
                    let gw = g.size().x;
                    painter.galley(pos2(cursor_x, center_y - g.size().y * 0.5), g, effective_fg);
                    cursor_x += gw + icon_gap;
                }
                let icon_rect = Rect::from_min_size(
                    pos2(cursor_x, center_y - icon_box * 0.5),
                    vec2(icon_box, icon_box),
                );
                paint_m3_icon(painter, icon, icon_rect, effective_fg, bg_behind_icon);
            }
        }
    }

    response
}

fn render_pill_button(
    ui: &mut Ui,
    text: &str,
    enabled: bool,
    min_size: Vec2,
    fill: Color32,
    fg: Color32,
    border: Option<Stroke>,
) -> Response {
    render_pill_button_with_icon(
        ui,
        text,
        IconPlacement::None,
        enabled,
        min_size,
        fill,
        fg,
        border,
    )
}

/// Renders a high-emphasis M3 Filled Button (`primary` container, `on_primary` label).
pub fn filled_button(ui: &mut Ui, text: &str, enabled: bool) -> Response {
    let p = palette(ui);
    render_pill_button(
        ui,
        text,
        enabled,
        vec2(96.0, 36.0),
        p.primary,
        p.on_primary,
        None,
    )
}

/// Renders a medium-emphasis M3 Filled Tonal Button (`secondary_container` fill,
/// `on_secondary_container` label).
pub fn tonal_button(ui: &mut Ui, text: &str, enabled: bool) -> Response {
    let p = palette(ui);
    render_pill_button(
        ui,
        text,
        enabled,
        vec2(92.0, 36.0),
        p.secondary_container,
        p.on_secondary_container,
        None,
    )
}

/// Renders a compact M3 Filled Tonal Button suitable for inline `"Browse..."` and `"Copy"` actions.
pub fn compact_tonal_button(ui: &mut Ui, text: &str, enabled: bool) -> Response {
    let p = palette(ui);
    render_pill_button(
        ui,
        text,
        enabled,
        vec2(82.0, 32.0),
        p.secondary_container,
        p.on_secondary_container,
        None,
    )
}

/// Renders a medium-emphasis M3 Outlined Button (`1dp` `outline` border, `primary` label).
pub fn outlined_button(ui: &mut Ui, text: &str, enabled: bool) -> Response {
    let p = palette(ui);
    render_pill_button(
        ui,
        text,
        enabled,
        vec2(92.0, 36.0),
        Color32::TRANSPARENT,
        p.primary,
        Some(Stroke::new(1.0, p.outline)),
    )
}

/// Renders a destructive M3 Outlined Button (`1dp` `error` border, `error` label).
pub fn danger_outlined_button(ui: &mut Ui, text: &str, enabled: bool) -> Response {
    let p = palette(ui);
    render_pill_button(
        ui,
        text,
        enabled,
        vec2(84.0, 32.0),
        Color32::TRANSPARENT,
        p.error,
        Some(Stroke::new(1.0, p.error)),
    )
}

/// Renders the M3 Light / Dark theme toggle button with a vector-painted Sun or Crescent Moon icon.
pub fn theme_toggle_button(ui: &mut Ui, current_mode: ThemeMode) -> Response {
    let p = palette(ui);
    let (icon, label) = if current_mode.is_dark() {
        (M3Icon::Sun, "Light")
    } else {
        (M3Icon::Moon, "Dark")
    };
    render_pill_button_with_icon(
        ui,
        label,
        IconPlacement::Leading(icon),
        true,
        vec2(92.0, 34.0),
        Color32::TRANSPARENT,
        p.primary,
        Some(Stroke::new(1.0, p.outline)),
    )
}

/// Renders a backward step navigation button (`Outlined` pill with a vector left-arrow icon).
pub fn nav_back_button(ui: &mut Ui, label: &str) -> Response {
    let p = palette(ui);
    render_pill_button_with_icon(
        ui,
        label,
        IconPlacement::Leading(M3Icon::ArrowLeft),
        true,
        vec2(116.0, 38.0),
        Color32::TRANSPARENT,
        p.primary,
        Some(Stroke::new(1.0, p.outline)),
    )
}

/// Renders a forward step navigation button (`Filled` pill with a vector right-arrow icon).
pub fn nav_next_button(ui: &mut Ui, label: &str) -> Response {
    let p = palette(ui);
    render_pill_button_with_icon(
        ui,
        label,
        IconPlacement::Trailing(M3Icon::ArrowRight),
        true,
        vec2(148.0, 38.0),
        p.primary,
        p.on_primary,
        None,
    )
}

/// Renders an animated Material Design 3 Switch (`48x28dp` stadium track with animated thumb
/// and vector-painted active checkmark).
pub fn m3_switch(ui: &mut Ui, on: &mut bool) -> Response {
    let p = palette(ui);
    let desired_size = vec2(48.0, 28.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, Sense::click());

    if response.clicked() && ui.is_enabled() {
        *on = !*on;
        response.mark_changed();
    }

    response.widget_info(|| WidgetInfo::selected(egui::Role::CheckBox, ui.is_enabled(), *on, ""));

    if response.hovered() && ui.is_enabled() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }

    if ui.is_rect_visible(rect) {
        let how_on = ui.ctx().animate_bool_responsive(response.id, *on);
        let radius = 0.5 * rect.height();

        let (track_fill, track_stroke, thumb_color) = if !ui.is_enabled() {
            (
                p.disabled_container(),
                Stroke::new(1.0, p.outline_variant),
                p.disabled_content(),
            )
        } else if *on {
            let base = if response.hovered() {
                M3Palette::state_layer(p.primary, p.on_primary, 0.10)
            } else {
                p.primary
            };
            (base, Stroke::NONE, p.on_primary)
        } else {
            let base = if response.hovered() {
                M3Palette::state_layer(p.surface_container_highest, p.on_surface, 0.08)
            } else {
                p.surface_container_highest
            };
            let thumb = if response.hovered() {
                p.on_surface_variant
            } else {
                p.outline
            };
            (base, Stroke::new(1.5, p.outline), thumb)
        };

        ui.painter()
            .rect_filled(rect, CornerRadius::same(radius as u8), track_fill);
        if track_stroke != Stroke::NONE {
            ui.painter().rect_stroke(
                rect,
                CornerRadius::same(radius as u8),
                track_stroke,
                StrokeKind::Inside,
            );
        }

        let start_x = rect.left() + radius;
        let end_x = rect.right() - radius;
        let center_x = egui::lerp(start_x..=end_x, how_on);
        let thumb_center = Pos2::new(center_x, rect.center().y);

        let base_thumb_radius = egui::lerp(7.5..=10.0, how_on);
        let thumb_radius = if response.is_pointer_button_down_on() && ui.is_enabled() {
            base_thumb_radius + 1.5
        } else {
            base_thumb_radius
        };

        ui.painter()
            .circle_filled(thumb_center, thumb_radius, thumb_color);

        if how_on > 0.65 {
            let check_color = if ui.is_enabled() {
                p.primary
            } else {
                p.disabled_container()
            };
            let pts = vec![
                thumb_center + vec2(-3.8, 0.2),
                thumb_center + vec2(-1.0, 3.0),
                thumb_center + vec2(4.2, -2.8),
            ];
            ui.painter()
                .add(Shape::line(pts, Stroke::new(1.8, check_color)));
        }
    }

    response
}

/// Renders a navigation arrow button using M3 Tonal styling and vector arrow icons.
pub fn nav_arrow(ui: &mut Ui, text: &str) -> Response {
    let p = palette(ui);
    let trimmed = text.trim();
    let placement = if trimmed == "<" || trimmed == "<-" || trimmed == "Back" {
        IconPlacement::Leading(M3Icon::ArrowLeft)
    } else {
        IconPlacement::Trailing(M3Icon::ArrowRight)
    };
    let clean_ascii: String = trimmed
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect();
    render_pill_button_with_icon(
        ui,
        &clean_ascii,
        placement,
        true,
        vec2(72.0, 36.0),
        p.secondary_container,
        p.on_secondary_container,
        None,
    )
}

/// Renders an M3 Tonal action button (retained for compatibility with existing callers).
pub fn blue_button(ui: &mut Ui, text: &str, enabled: bool) -> Response {
    tonal_button(ui, text, enabled)
}

/// Renders the `"Add Destination"` button on the Output tab with a vector plus icon.
pub fn plus_button(ui: &mut Ui, enabled: bool) -> Response {
    let p = palette(ui);
    render_pill_button_with_icon(
        ui,
        "Add Destination",
        IconPlacement::Leading(M3Icon::Plus),
        enabled,
        vec2(156.0, 38.0),
        p.secondary_container,
        p.on_secondary_container,
        None,
    )
}

/// Renders the remove-destination button on the Output tab using M3 Outlined Error styling.
pub fn minus_button(ui: &mut Ui) -> Response {
    danger_outlined_button(ui, "Remove", true)
}

/// Renders the `"Stop Datapipe"` M3 Filled Error button with a vector stop-square icon.
pub fn stop_button(ui: &mut Ui, enabled: bool) -> Response {
    let p = palette(ui);
    render_pill_button_with_icon(
        ui,
        "Stop Datapipe",
        IconPlacement::Leading(M3Icon::Stop),
        enabled,
        vec2(162.0, 40.0),
        p.error,
        p.on_error,
        None,
    )
}

/// Renders the `"Start Datapipe"` M3 Filled Tertiary button with a vector play-triangle icon.
pub fn start_button(ui: &mut Ui, enabled: bool) -> Response {
    let p = palette(ui);
    render_pill_button_with_icon(
        ui,
        "Start Datapipe",
        IconPlacement::Leading(M3Icon::Play),
        enabled,
        vec2(162.0, 40.0),
        p.tertiary,
        p.on_tertiary,
        None,
    )
}

/// Transient per-widget clipboard/context-menu state stored in [`egui::Context`] memory so that
/// right-clicking a highlighted selection never collapses the selection and `Paste` via
/// [`ViewportCommand::RequestPaste`] retains focus and selection across frame boundaries.
#[derive(Debug, Clone, Copy, Default)]
struct TextFieldMenuState {
    preserved_cursor: Option<CCursorRange>,
    secondary_hold: bool,
    pending_paste_frames: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClipboardAction {
    Cut,
    Copy,
    Paste,
    SelectAll,
}

fn clamp_cursor_range(range: Option<CCursorRange>, text: &str) -> Option<CCursorRange> {
    let mut r = range?;
    let max_chars = text.chars().count();
    if r.primary.index.0 > max_chars {
        r.primary = CCursor::new(max_chars);
    }
    if r.secondary.index.0 > max_chars {
        r.secondary = CCursor::new(max_chars);
    }
    Some(r)
}

fn store_cursor_range(ctx: &egui::Context, id: Id, range: CCursorRange) {
    let mut state = TextEditState::load(ctx, id).unwrap_or_default();
    state.cursor.set_char_range(Some(range));
    state.store(ctx, id);
}

fn execute_clipboard_action(
    ctx: &egui::Context,
    id: Id,
    value: &mut String,
    state: &mut TextEditState,
    menu_state: &mut TextFieldMenuState,
    active_range: CCursorRange,
    action: ClipboardAction,
) -> bool {
    match action {
        ClipboardAction::Cut => {
            if active_range.is_empty() {
                false
            } else {
                let selected = active_range.slice_str(value.as_str()).to_owned();
                ctx.copy_text(selected);
                let new_cursor = value.delete_selected(&active_range);
                let new_range = CCursorRange::one(new_cursor);
                state.cursor.set_char_range(Some(new_range));
                state.clone().store(ctx, id);
                menu_state.preserved_cursor = None;
                ctx.memory_mut(|mem| mem.request_focus(id));
                ctx.request_repaint();
                true
            }
        }
        ClipboardAction::Copy => {
            if !active_range.is_empty() {
                let selected = active_range.slice_str(value.as_str()).to_owned();
                ctx.copy_text(selected);
                state.cursor.set_char_range(Some(active_range));
                state.clone().store(ctx, id);
                menu_state.preserved_cursor = None;
                ctx.memory_mut(|mem| mem.request_focus(id));
                ctx.request_repaint();
            }
            false
        }
        ClipboardAction::Paste => {
            state.cursor.set_char_range(Some(active_range));
            state.clone().store(ctx, id);
            menu_state.preserved_cursor = Some(active_range);
            menu_state.pending_paste_frames = 3;
            ctx.memory_mut(|mem| mem.request_focus(id));
            ctx.send_viewport_cmd(ViewportCommand::RequestPaste);
            ctx.request_repaint();
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
            false
        }
        ClipboardAction::SelectAll => {
            let all_range = CCursorRange::select_all_str(value.as_str());
            state.cursor.set_char_range(Some(all_range));
            state.clone().store(ctx, id);
            menu_state.preserved_cursor = None;
            ctx.memory_mut(|mem| mem.request_focus(id));
            ctx.request_repaint();
            false
        }
    }
}

fn context_menu_item(ui: &mut Ui, label: &str, shortcut: &str, enabled: bool) -> Response {
    let p = palette(ui);
    let is_enabled = enabled && ui.is_enabled();
    let desired_size = vec2(ui.available_width().max(184.0), 30.0);
    let sense = if is_enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(desired_size, sense);

    response.widget_info(|| WidgetInfo::labeled(egui::Role::Button, is_enabled, label));

    if is_enabled && response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = CornerRadius::same(6);

        if is_enabled {
            if response.is_pointer_button_down_on() {
                let fill = M3Palette::state_layer(p.surface_container, p.on_surface, 0.12);
                painter.rect_filled(rect, radius, fill);
            } else if response.hovered() {
                let fill = M3Palette::state_layer(p.surface_container, p.on_surface, 0.08);
                painter.rect_filled(rect, radius, fill);
            }
        }

        let label_color = if is_enabled {
            p.on_surface
        } else {
            p.disabled_content()
        };
        let shortcut_color = if is_enabled {
            p.on_surface_variant
        } else {
            p.disabled_content()
        };

        let label_galley = painter.layout_no_wrap(
            label.to_string(),
            FontId::new(13.5, FontFamily::Proportional),
            label_color,
        );
        let shortcut_galley = painter.layout_no_wrap(
            shortcut.to_string(),
            FontId::new(11.5, FontFamily::Proportional),
            shortcut_color,
        );

        let pad_x = 12.0;
        painter.galley(
            pos2(
                rect.left() + pad_x,
                rect.center().y - label_galley.size().y * 0.5,
            ),
            label_galley,
            label_color,
        );
        painter.galley(
            pos2(
                rect.right() - pad_x - shortcut_galley.size().x,
                rect.center().y - shortcut_galley.size().y * 0.5,
            ),
            shortcut_galley,
            shortcut_color,
        );
    }

    response
}

fn render_m3_text_field(
    ui: &mut Ui,
    value: &mut String,
    hint: &str,
    desired_width: f32,
    enabled: bool,
    monospace: bool,
    masked: bool,
) -> Response {
    ui.add_enabled_ui(enabled, |ui| {
        let id = ui.next_auto_id();
        ui.skip_ahead_auto_ids(1);

        let popup_id = id.with("popup");
        let menu_state_id = id.with("m3_text_menu_state");

        let mut menu_state = ui
            .ctx()
            .data_mut(|d| d.get_temp::<TextFieldMenuState>(menu_state_id))
            .unwrap_or_default();

        let popup_was_open = Popup::is_id_open(ui.ctx(), popup_id);
        let popup_rect = ui.memory(|mem| mem.area_rect(popup_id));

        let (secondary_pressed, secondary_down, primary_pressed, pointer_pos, has_paste_event) = ui
            .input(|i| {
                (
                    i.pointer.button_pressed(PointerButton::Secondary),
                    i.pointer.button_down(PointerButton::Secondary),
                    i.pointer.button_pressed(PointerButton::Primary),
                    i.pointer.interact_pos().or_else(|| i.pointer.latest_pos()),
                    i.events.iter().any(|e| matches!(e, Event::Paste(_))),
                )
            });

        let clicking_outside_popup = primary_pressed
            && popup_rect.is_some_and(|rect| pointer_pos.is_some_and(|pos| !rect.contains(pos)));

        let pre_cursor = TextEditState::load(ui.ctx(), id)
            .and_then(|s| clamp_cursor_range(s.cursor.char_range(), value));

        // 1. Pre-show: preserve focus and selection across pending RequestPaste frames,
        //    secondary-button hold, or while the right-click context menu is open.
        if menu_state.pending_paste_frames > 0 {
            ui.memory_mut(|mem| mem.request_focus(id));
            if let Some(saved) = clamp_cursor_range(menu_state.preserved_cursor, value) {
                store_cursor_range(ui.ctx(), id, saved);
            }
            if has_paste_event {
                menu_state.pending_paste_frames = 0;
                menu_state.preserved_cursor = None;
            } else {
                menu_state.pending_paste_frames -= 1;
                ui.ctx().request_repaint();
            }
        } else if menu_state.secondary_hold && secondary_down {
            if let Some(saved) = clamp_cursor_range(menu_state.preserved_cursor, value) {
                ui.memory_mut(|mem| mem.request_focus(id));
                store_cursor_range(ui.ctx(), id, saved);
            }
        } else if popup_was_open
            && !clicking_outside_popup
            && let Some(saved) = clamp_cursor_range(menu_state.preserved_cursor, value)
        {
            if ui.memory(|mem| mem.focused()).is_none_or(|f| f == id) {
                ui.memory_mut(|mem| mem.request_focus(id));
            }
            store_cursor_range(ui.ctx(), id, saved);
        }

        // 2. Render the underlying single-line TextEdit widget.
        let p = palette(ui);
        let mask_font_id = if monospace {
            TextStyle::Monospace.resolve(ui.style())
        } else {
            TextStyle::Body.resolve(ui.style())
        };
        let mask_line_height =
            ui.fonts_mut(|f| f.row_height(&mask_font_id)) + ui.spacing().extra_text_line_spacing;
        let mask_text_color = if ui.is_enabled() {
            ui.style().visuals.text_color()
        } else {
            p.disabled_content()
        };
        // Mask with ASCII '*' via layouter rather than `.password(true)` so secret fields
        // render strictly ASCII glyphs and retain Cut/Copy clipboard behavior.
        let mut mask_layouter = |ui: &Ui, buf: &dyn egui::TextBuffer, _wrap_width: f32| {
            let masked_text = "*".repeat(buf.as_str().chars().count());
            let mut layout_job = egui::text::LayoutJob::simple_singleline(
                masked_text,
                mask_font_id.clone(),
                mask_text_color,
            );
            layout_job.keep_trailing_whitespace = true;
            for section in &mut layout_job.sections {
                section.format.line_height = Some(mask_line_height);
            }
            ui.fonts_mut(|f| f.layout_job(layout_job))
        };

        let mut text_edit = egui::TextEdit::singleline(value)
            .id(id)
            .hint_text(hint)
            .desired_width(desired_width);
        if monospace {
            text_edit = text_edit.font(TextStyle::Monospace);
        }
        if masked {
            text_edit = text_edit.layouter(&mut mask_layouter);
        }

        let mut output = text_edit.show(ui);
        let mut response = output.response.response;

        // 3. Post-show: prevent right-click press/drag/release from collapsing an existing
        //    highlighted text selection.
        if secondary_pressed && response.hovered() {
            let prior_selection = pre_cursor.filter(|r| !r.is_empty()).or_else(|| {
                popup_was_open
                    .then(|| clamp_cursor_range(menu_state.preserved_cursor, value))
                    .flatten()
                    .filter(|r| !r.is_empty())
            });

            if let Some(sel) = prior_selection {
                menu_state.preserved_cursor = Some(sel);
                menu_state.secondary_hold = true;
                output.state.cursor.set_char_range(Some(sel));
                output.state.clone().store(ui.ctx(), id);
                ui.memory_mut(|mem| mem.request_focus(id));
                ui.ctx().request_repaint();
            } else {
                menu_state.preserved_cursor =
                    clamp_cursor_range(output.state.cursor.char_range(), value);
                menu_state.secondary_hold = true;
            }
        } else if menu_state.secondary_hold && secondary_down {
            if let Some(saved) = clamp_cursor_range(menu_state.preserved_cursor, value) {
                output.state.cursor.set_char_range(Some(saved));
                output.state.clone().store(ui.ctx(), id);
                ui.memory_mut(|mem| mem.request_focus(id));
            }
        } else if response.secondary_clicked() {
            if !menu_state.secondary_hold {
                menu_state.preserved_cursor = pre_cursor
                    .filter(|r| !r.is_empty())
                    .or_else(|| clamp_cursor_range(output.state.cursor.char_range(), value));
            }
            menu_state.secondary_hold = false;
            if let Some(saved) = clamp_cursor_range(menu_state.preserved_cursor, value) {
                output.state.cursor.set_char_range(Some(saved));
                output.state.clone().store(ui.ctx(), id);
                ui.memory_mut(|mem| mem.request_focus(id));
            }
        } else if !secondary_down {
            menu_state.secondary_hold = false;
        }

        // 4. Fallback keyboard shortcut handling for Ctrl+X / Ctrl+C / Ctrl+V / Ctrl+A when
        //    raw Event::Key events arrive without prior Event::Cut / Copy / Paste translation
        //    (e.g., physical Ctrl shortcuts on macOS or direct Event::Key injection).
        if ui.is_enabled() && ui.memory(|mem| mem.has_focus(id)) {
            let fallback_action = ui.input(|i| {
                let has_native_clipboard_event = i
                    .events
                    .iter()
                    .any(|e| matches!(e, Event::Copy | Event::Cut | Event::Paste(_)));
                if has_native_clipboard_event {
                    return None;
                }
                i.events.iter().find_map(|event| match event {
                    Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } if (modifiers.ctrl || modifiers.command)
                        && !modifiers.alt
                        && !modifiers.shift =>
                    {
                        match key {
                            Key::X => Some(ClipboardAction::Cut),
                            Key::C => Some(ClipboardAction::Copy),
                            Key::V => Some(ClipboardAction::Paste),
                            Key::A => Some(ClipboardAction::SelectAll),
                            _ => None,
                        }
                    }
                    _ => None,
                })
            });

            if let Some(action) = fallback_action {
                let active_range = clamp_cursor_range(output.state.cursor.char_range(), value)
                    .unwrap_or_else(|| CCursorRange::one(CCursor::end_of_str(value)));
                if execute_clipboard_action(
                    ui.ctx(),
                    id,
                    value,
                    &mut output.state,
                    &mut menu_state,
                    active_range,
                    action,
                ) {
                    response.mark_changed();
                }
            }
        }

        // 5. Material Design 3 right-click pop-up context menu with pure-ASCII labels & hints.
        let mut menu_mutated_text = false;
        if ui.is_enabled() {
            response.context_menu(|ui| {
                ui.set_min_width(184.0);
                ui.spacing_mut().item_spacing.y = 2.0;

                let active_range = clamp_cursor_range(
                    menu_state
                        .preserved_cursor
                        .or_else(|| output.state.cursor.char_range()),
                    value,
                )
                .unwrap_or_else(|| CCursorRange::one(CCursor::end_of_str(value)));

                let has_selection = !active_range.is_empty();
                let has_text = !value.is_empty();

                if context_menu_item(ui, "Cut", "Ctrl+X", has_selection).clicked() {
                    menu_mutated_text |= execute_clipboard_action(
                        ui.ctx(),
                        id,
                        value,
                        &mut output.state,
                        &mut menu_state,
                        active_range,
                        ClipboardAction::Cut,
                    );
                    ui.close();
                }

                if context_menu_item(ui, "Copy", "Ctrl+C", has_selection).clicked() {
                    menu_mutated_text |= execute_clipboard_action(
                        ui.ctx(),
                        id,
                        value,
                        &mut output.state,
                        &mut menu_state,
                        active_range,
                        ClipboardAction::Copy,
                    );
                    ui.close();
                }

                if context_menu_item(ui, "Paste", "Ctrl+V", true).clicked() {
                    menu_mutated_text |= execute_clipboard_action(
                        ui.ctx(),
                        id,
                        value,
                        &mut output.state,
                        &mut menu_state,
                        active_range,
                        ClipboardAction::Paste,
                    );
                    ui.close();
                }

                ui.add_space(2.0);
                ui.separator();
                ui.add_space(2.0);

                if context_menu_item(ui, "Select All", "Ctrl+A", has_text).clicked() {
                    menu_mutated_text |= execute_clipboard_action(
                        ui.ctx(),
                        id,
                        value,
                        &mut output.state,
                        &mut menu_state,
                        active_range,
                        ClipboardAction::SelectAll,
                    );
                    ui.close();
                }
            });
        }
        if menu_mutated_text {
            response.mark_changed();
        }

        let popup_is_open = response.context_menu_opened();
        if !popup_is_open && !menu_state.secondary_hold && menu_state.pending_paste_frames == 0 {
            menu_state.preserved_cursor = None;
        }

        ui.ctx()
            .data_mut(|d| d.insert_temp(menu_state_id, menu_state));

        response
    })
    .inner
}

/// Renders an enabled single-line Material Design 3 text field supporting full text selection,
/// standard `Ctrl+X` / `Ctrl+C` / `Ctrl+V` / `Ctrl+A` keyboard shortcuts, and a right-click
/// pop-up context menu (`Cut`, `Copy`, `Paste`, `Select All`).
///
/// # Examples
/// ```
/// use datapipe::gui::style::m3_text_field;
///
/// let ctx = egui::Context::default();
/// let mut url = String::from("http://127.0.0.1:8080");
/// let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
///     let _response = m3_text_field(ui, &mut url, "http://example.com", 240.0);
/// });
/// output.textures_delta.clear();
/// ```
pub fn m3_text_field(ui: &mut Ui, value: &mut String, hint: &str, desired_width: f32) -> Response {
    render_m3_text_field(ui, value, hint, desired_width, true, false, false)
}

/// Renders a single-line Material Design 3 text field with explicit `enabled` control,
/// supporting text highlighting, `Ctrl+X` / `Ctrl+C` / `Ctrl+V` / `Ctrl+A` shortcuts, and a
/// right-click pop-up context menu (`Cut`, `Copy`, `Paste`, `Select All`) when enabled.
///
/// # Examples
/// ```
/// use datapipe::gui::style::m3_text_field_enabled;
///
/// let ctx = egui::Context::default();
/// let mut path = String::from("/tmp/cert.pem");
/// let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
///     let _response = m3_text_field_enabled(
///         ui,
///         &mut path,
///         "Certificate (PEM or DER)",
///         220.0,
///         true,
///     );
/// });
/// output.textures_delta.clear();
/// ```
pub fn m3_text_field_enabled(
    ui: &mut Ui,
    value: &mut String,
    hint: &str,
    desired_width: f32,
    enabled: bool,
) -> Response {
    render_m3_text_field(ui, value, hint, desired_width, enabled, false, false)
}

/// Renders a single-line monospace Material Design 3 text field (suitable for hexadecimal keys,
/// peer IDs, and hashes) with explicit `enabled` control, supporting text highlighting,
/// `Ctrl+X` / `Ctrl+C` / `Ctrl+V` / `Ctrl+A` shortcuts, and a right-click pop-up context menu.
///
/// # Examples
/// ```
/// use datapipe::gui::style::m3_monospace_field;
///
/// let ctx = egui::Context::default();
/// let mut key = String::new();
/// let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
///     let _response = m3_monospace_field(ui, &mut key, "Enter Peer EndpointId...", 320.0, true);
/// });
/// output.textures_delta.clear();
/// ```
pub fn m3_monospace_field(
    ui: &mut Ui,
    value: &mut String,
    hint: &str,
    desired_width: f32,
    enabled: bool,
) -> Response {
    render_m3_text_field(ui, value, hint, desired_width, enabled, true, false)
}

/// Renders a single-line masked monospace Material Design 3 secret field with an adjacent
/// `Show`/`Hide` toggle button. Supports full text selection, `Ctrl+X` / `Ctrl+C` / `Ctrl+V` /
/// `Ctrl+A` clipboard shortcuts (even while masked), and a right-click pop-up context menu.
///
/// # Examples
/// ```
/// use datapipe::gui::style::m3_secret_field;
///
/// let ctx = egui::Context::default();
/// let mut secret = String::new();
/// let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
///     let _response = m3_secret_field(
///         ui,
///         "secret_example",
///         &mut secret,
///         "Enter encryption key...",
///         320.0,
///         true,
///     );
/// });
/// output.textures_delta.clear();
/// ```
pub fn m3_secret_field(
    ui: &mut Ui,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
    value: &mut String,
    hint: &str,
    desired_width: f32,
    enabled: bool,
) -> Response {
    let reveal_id = ui.make_persistent_id(("m3_secret_reveal", id_salt));
    let mut revealed = ui
        .ctx()
        .data_mut(|d| d.get_temp::<bool>(reveal_id).unwrap_or(false));

    ui.horizontal(|ui| {
        let toggle_width = 64.0;
        let field_width = (desired_width - toggle_width - ui.spacing().item_spacing.x).max(120.0);
        let response = render_m3_text_field(ui, value, hint, field_width, enabled, true, !revealed);
        let p = palette(ui);
        let label = if revealed { "Hide" } else { "Show" };
        let (fill, text_color) = if revealed {
            (p.primary_container, p.on_primary_container)
        } else {
            (p.secondary_container, p.on_secondary_container)
        };
        if render_pill_button(
            ui,
            label,
            enabled,
            vec2(toggle_width, 32.0),
            fill,
            text_color,
            None,
        )
        .clicked()
        {
            revealed = !revealed;
            ui.ctx().data_mut(|d| d.insert_temp(reveal_id, revealed));
        }
        response
    })
    .inner
}

/// Renders a standard muted field label for two-column form grids.
pub(crate) fn m3_field_label(ui: &mut Ui, label: &str) {
    let p = palette(ui);
    ui.label(
        RichText::new(label)
            .size(13.5)
            .strong()
            .color(p.on_surface_variant),
    );
}

/// Renders a security-sensitive checkbox accompanied by an `INSECURE` warning chip when checked.
pub(crate) fn m3_insecure_checkbox(ui: &mut Ui, checked: &mut bool, label: &str) {
    let p = palette(ui);
    ui.horizontal(|ui| {
        ui.checkbox(checked, label);
        if *checked {
            status_chip(ui, "INSECURE", p.warning_container, p.on_warning_container);
        }
    });
}

/// Renders the shared key-validation progress bar and status chip for Encrypt/Decrypt tabs.
pub(crate) fn m3_encryption_key_status(
    ui: &mut Ui,
    key: &str,
    ready_message: &str,
    add_trailing: impl FnOnce(&mut Ui),
) {
    let p = palette(ui);
    let trimmed_empty = key.trim().is_empty();
    let is_valid = !trimmed_empty && EncryptionKey::new(key).is_ok();
    let (progress, bar_color) = if is_valid {
        (1.0, p.tertiary)
    } else if trimmed_empty {
        (0.0, p.error)
    } else {
        (0.5, p.error)
    };

    ui.add(
        ProgressBar::new(progress)
            .desired_height(6.0)
            .corner_radius(CornerRadius::same(3))
            .fill(bar_color),
    );

    ui.add_space(8.0);

    ui.horizontal_wrapped(|ui| {
        if is_valid {
            status_chip(
                ui,
                "VALID KEY",
                p.tertiary_container,
                p.on_tertiary_container,
            );
            ui.label(
                RichText::new(ready_message)
                    .size(12.5)
                    .color(p.on_surface_variant),
            );
        } else if trimmed_empty {
            status_chip(ui, "KEY REQUIRED", p.error_container, p.on_error_container);
            ui.label(
                RichText::new("Enter or generate a dpk1:, hex:, or legacy 51-byte UTF-8 key.")
                    .size(12.5)
                    .color(p.error),
            );
        } else {
            status_chip(ui, "INVALID KEY", p.error_container, p.on_error_container);
            ui.label(
                RichText::new(
                    "Key must be dpk1:<68 base64url>, hex:<102 hex>, or legacy 51 UTF-8 bytes.",
                )
                .size(12.5)
                .color(p.error),
            );
        }
        add_trailing(ui);
    });
}

/// Renders the standardized M3 Bottom Action Bar inside a tab view.
pub fn bottom_action_bar(ui: &mut Ui, id_salt: &'static str, add_contents: impl FnOnce(&mut Ui)) {
    let p = palette(ui);
    let bar_frame = Frame::NONE
        .fill(p.surface_container_low)
        .inner_margin(Margin::symmetric(24, 12));

    egui::Panel::bottom(id_salt)
        .resizable(false)
        .show_separator_line(false)
        .frame(bar_frame)
        .show(ui, |ui| {
            let rect = ui.max_rect();
            // Crisp 1dp top divider across the full footer bar
            let divider_y = rect.min.y - 11.5;
            ui.painter().line_segment(
                [
                    pos2(rect.min.x - 24.0, divider_y),
                    pos2(rect.max.x + 24.0, divider_y),
                ],
                Stroke::new(1.0, p.outline_variant),
            );
            add_contents(ui);
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_mode_toggle_and_display() {
        let mut mode = ThemeMode::default();
        assert_eq!(mode, ThemeMode::Light);
        assert!(!mode.is_dark());
        assert_eq!(mode.to_string(), "Light");
        assert_eq!(mode.to_egui_theme(), Theme::Light);

        mode.toggle();
        assert_eq!(mode, ThemeMode::Dark);
        assert!(mode.is_dark());
        assert_eq!(mode.to_string(), "Dark");
        assert_eq!(mode.to_egui_theme(), Theme::Dark);

        mode.toggle();
        assert_eq!(mode, ThemeMode::Light);
    }

    #[test]
    fn test_m3_palettes_distinct_and_high_contrast() {
        let light = M3Palette::light();
        let dark = M3Palette::dark();

        assert_ne!(light, dark);
        assert_ne!(light.surface, dark.surface);
        assert_ne!(light.primary, dark.primary);
        assert_ne!(light.on_surface, light.surface);
        assert_ne!(dark.on_surface, dark.surface);
        assert_eq!(M3Palette::for_mode(ThemeMode::Light), light);
        assert_eq!(M3Palette::for_mode(ThemeMode::Dark), dark);
    }

    #[test]
    fn test_apply_theme_updates_context_visuals() {
        let ctx = egui::Context::default();
        apply_theme(&ctx, ThemeMode::Light);
        assert_eq!(ctx.theme(), Theme::Light);
        assert!(!ctx.global_style().visuals.dark_mode);

        apply_theme(&ctx, ThemeMode::Dark);
        assert_eq!(ctx.theme(), Theme::Dark);
        assert!(ctx.global_style().visuals.dark_mode);
    }
}
