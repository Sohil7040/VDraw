//! Theme tokens and visuals builder for VDraw.
#![allow(dead_code)]

// ─── Color palette ───────────────────────────────────────────────────────────

pub const BG_BASE: egui::Color32 = egui::Color32::from_rgb(0x16, 0x18, 0x1D);
pub const BG_PANEL: egui::Color32 = egui::Color32::from_rgb(0x1E, 0x21, 0x28);
pub const BG_WIDGET: egui::Color32 = egui::Color32::from_rgb(0x26, 0x2A, 0x33);
pub const BG_HOVER: egui::Color32 = egui::Color32::from_rgb(0x2E, 0x33, 0x3E);
pub const BG_ACTIVE: egui::Color32 = egui::Color32::from_rgb(0x35, 0x3B, 0x48);
pub const ACCENT: egui::Color32 = egui::Color32::from_rgb(0x7C, 0x5C, 0xFF);
pub const ACCENT_HOVER: egui::Color32 = egui::Color32::from_rgb(0x9B, 0x80, 0xFF);
pub const TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(0xE8, 0xE9, 0xEC);
pub const TEXT_SECONDARY: egui::Color32 = egui::Color32::from_rgb(0x8B, 0x91, 0xA1);
pub const TEXT_DISABLED: egui::Color32 = egui::Color32::from_rgb(0x4E, 0x55, 0x65);
pub const BORDER: egui::Color32 = egui::Color32::from_rgb(0x2E, 0x33, 0x3E);
pub const BORDER_FOCUS: egui::Color32 = egui::Color32::from_rgb(0x7C, 0x5C, 0xFF);
pub const CANVAS_BG: egui::Color32 = egui::Color32::from_rgb(0x0E, 0x10, 0x14);
pub const PAGE_BG: egui::Color32 = egui::Color32::from_rgb(0xFF, 0xFF, 0xFF);
pub const RULER_BG: egui::Color32 = egui::Color32::from_rgb(0x1E, 0x21, 0x28);
pub const RULER_TICK: egui::Color32 = egui::Color32::from_rgb(0x4E, 0x55, 0x65);
pub const RULER_TEXT: egui::Color32 = egui::Color32::from_rgb(0x8B, 0x91, 0xA1);
pub const GRID_LINE: egui::Color32 = egui::Color32::from_rgba_premultiplied(0x2E, 0x33, 0x3E, 0x80);
pub const GUIDE_LINE: egui::Color32 = egui::Color32::from_rgb(0x20, 0xD3, 0xB5);
pub const SELECTION_FILL: egui::Color32 =
    egui::Color32::from_rgba_premultiplied(0x7C, 0x5C, 0xFF, 0x28);
pub const SELECTION_STROKE: egui::Color32 = egui::Color32::from_rgb(0x7C, 0x5C, 0xFF);

// ─── Spacing & sizing tokens ─────────────────────────────────────────────────

pub const SPACING: f32 = 8.0;
pub const CORNER_RADIUS: u8 = 6;
pub const TOOLBAR_WIDTH: f32 = 48.0;
pub const RULER_SIZE: f32 = 20.0;
pub const STATUSBAR_HEIGHT: f32 = 24.0;
pub const PROPBAR_HEIGHT: f32 = 36.0;

// ─── Visuals builder ─────────────────────────────────────────────────────────

/// Build an `egui::Visuals` using VDraw's dark-mode theme tokens.
pub fn build_visuals(dark: bool) -> egui::Visuals {
    if !dark {
        // Fall back to egui's built-in light theme for now.
        return egui::Visuals::light();
    }

    let mut v = egui::Visuals::dark();

    // Window / panel backgrounds
    v.window_fill = BG_PANEL;
    v.panel_fill = BG_PANEL;

    // Widgets
    v.widgets.noninteractive.bg_fill = BG_PANEL;
    v.widgets.noninteractive.weak_bg_fill = BG_PANEL;
    v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, BORDER);
    v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, TEXT_SECONDARY);
    v.widgets.noninteractive.corner_radius = egui::CornerRadius::same(CORNER_RADIUS);

    v.widgets.inactive.bg_fill = BG_WIDGET;
    v.widgets.inactive.weak_bg_fill = BG_WIDGET;
    v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, BORDER);
    v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, TEXT_PRIMARY);
    v.widgets.inactive.corner_radius = egui::CornerRadius::same(CORNER_RADIUS);

    v.widgets.hovered.bg_fill = BG_HOVER;
    v.widgets.hovered.weak_bg_fill = BG_HOVER;
    v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, ACCENT_HOVER);
    v.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, TEXT_PRIMARY);
    v.widgets.hovered.corner_radius = egui::CornerRadius::same(CORNER_RADIUS);

    v.widgets.active.bg_fill = BG_ACTIVE;
    v.widgets.active.weak_bg_fill = BG_ACTIVE;
    v.widgets.active.bg_stroke = egui::Stroke::new(1.5, ACCENT);
    v.widgets.active.fg_stroke = egui::Stroke::new(1.5, TEXT_PRIMARY);
    v.widgets.active.corner_radius = egui::CornerRadius::same(CORNER_RADIUS);

    v.widgets.open.bg_fill = BG_ACTIVE;
    v.widgets.open.weak_bg_fill = BG_ACTIVE;
    v.widgets.open.bg_stroke = egui::Stroke::new(1.0, ACCENT);
    v.widgets.open.fg_stroke = egui::Stroke::new(1.0, TEXT_PRIMARY);
    v.widgets.open.corner_radius = egui::CornerRadius::same(CORNER_RADIUS);

    // Selection
    v.selection.bg_fill = SELECTION_FILL;
    v.selection.stroke = egui::Stroke::new(1.5, SELECTION_STROKE);

    // Hyperlinks
    v.hyperlink_color = ACCENT_HOVER;

    // Window corner radius & shadow
    v.window_corner_radius = egui::CornerRadius::same(CORNER_RADIUS);
    v.window_shadow = egui::Shadow {
        offset: [0, 4],
        blur: 16,
        spread: 0,
        color: egui::Color32::from_black_alpha(120),
    };

    // Separators / strokes
    v.window_stroke = egui::Stroke::new(1.0, BORDER);

    v
}

/// Apply VDraw's spacing style to a `egui::Style`.
pub fn apply_style(style: &mut egui::Style) {
    style.spacing.item_spacing = egui::vec2(SPACING, SPACING * 0.5);
    style.spacing.button_padding = egui::vec2(SPACING, SPACING * 0.5);
    style.spacing.indent = SPACING * 2.0;
    style.spacing.scroll.bar_width = 6.0;
}
