//! Semantic design tokens for VDraw (`ThemeTokens`, 8 px grid, Dark/Light palettes, Okabe–Ito snap colors).

use core_snap::contracts::SnapKind;
pub use core_tools::ThemePref;
use egui::{Color32, CornerRadius, Shadow, Stroke, ThemePreference, Visuals};

const STORAGE_THEME_KEY: &str = "vdraw.appearance.theme_pref";

/// 8 px base spacing grid tokens.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SpacingTokens;

impl SpacingTokens {
    pub const GRID: f32 = 8.0;
    pub const XS: f32 = 4.0;
    pub const SM: f32 = 8.0;
    pub const MD: f32 = 16.0;
    pub const LG: f32 = 24.0;
    pub const XL: f32 = 32.0;

    pub const TOOLBAR_WIDTH: f32 = 128.0;
    pub const STATUSBAR_HEIGHT: f32 = 24.0;
    pub const COMMANDBAR_HEIGHT: f32 = 40.0;
    pub const RULER_SIZE: f32 = 24.0;
    pub const INSPECTOR_WIDTH: f32 = 280.0;
}

/// Corner radius tokens aligned with the spacing scale.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RadiusTokens;

impl RadiusTokens {
    pub const SM: u8 = 4;
    pub const MD: u8 = 6;
    pub const LG: u8 = 10;
}

/// Typography sizing tokens.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TypographyTokens;

impl TypographyTokens {
    pub const CAPTION: f32 = 11.0;
    pub const BODY: f32 = 13.0;
    pub const SUBHEADING: f32 = 14.5;
    pub const HEADING: f32 = 17.0;
}

/// Semantic color palette supporting both Dark and Light themes plus colorblind-safe Okabe–Ito snap colors.
#[derive(Debug, Clone, PartialEq)]
pub struct SemanticColors {
    pub surface: Color32,
    pub surface_elevated: Color32,
    pub surface_interactive: Color32,
    pub surface_hover: Color32,
    pub surface_active: Color32,
    pub canvas_bg: Color32,
    pub page_bg: Color32,
    pub on_surface: Color32,
    pub on_surface_muted: Color32,
    pub on_surface_disabled: Color32,
    pub border: Color32,
    pub border_focus: Color32,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub on_accent: Color32,
    pub danger: Color32,
    pub warning: Color32,
    pub success: Color32,
    pub grid_line: Color32,
    pub grid_major_line: Color32,
    pub guide_line: Color32,
    pub ruler_tick: Color32,
    pub align_guide: Color32,
    pub polar_guide: Color32,
    pub selection_fill: Color32,
    pub selection_stroke: Color32,
    pub page_border: Color32,
    pub page_shadow: Color32,
    pub backdrop_scrim: Color32,
    pub hud_pill_bg: Color32,
    pub laser_cut: Color32,
    pub laser_score: Color32,
    pub laser_active: Color32,
    pub transparent: Color32,
    // Okabe–Ito colorblind-safe snap colors (WCAG AA contrast >= 4.5:1 on surface):
    pub snap_node: Color32,
    pub snap_center: Color32,
    pub snap_endpoint: Color32,
    pub snap_midpoint: Color32,
    pub snap_quadrant: Color32,
    pub snap_intersection: Color32,
    pub snap_grid: Color32,
    pub snap_perpendicular: Color32,
    pub snap_tangent: Color32,
}

impl SemanticColors {
    /// Returns the dark palette (WCAG AA compliant against `#16181D` / `#1E2128`).
    pub const fn dark() -> Self {
        Self {
            surface: Color32::from_rgb(0x16, 0x18, 0x1D),
            surface_elevated: Color32::from_rgb(0x1E, 0x21, 0x28),
            surface_interactive: Color32::from_rgb(0x26, 0x2A, 0x33),
            surface_hover: Color32::from_rgb(0x2E, 0x33, 0x3E),
            surface_active: Color32::from_rgb(0x35, 0x3B, 0x48),
            canvas_bg: Color32::from_rgb(0x0E, 0x10, 0x14),
            page_bg: Color32::from_rgb(0xFF, 0xFF, 0xFF),
            on_surface: Color32::from_rgb(0xEE, 0xF0, 0xF4),
            on_surface_muted: Color32::from_rgb(0xA4, 0xAA, 0xB8),
            on_surface_disabled: Color32::from_rgb(0x63, 0x6A, 0x7A),
            border: Color32::from_rgb(0x32, 0x38, 0x44),
            border_focus: Color32::from_rgb(0x56, 0xB4, 0xE9),
            accent: Color32::from_rgb(0x7C, 0x5C, 0xFF),
            accent_hover: Color32::from_rgb(0x9B, 0x80, 0xFF),
            on_accent: Color32::from_rgb(0xFF, 0xFF, 0xFF),
            danger: Color32::from_rgb(0xFF, 0x6B, 0x6B),
            warning: Color32::from_rgb(0xE6, 0x9F, 0x00),
            success: Color32::from_rgb(0x00, 0x9E, 0x73),
            grid_line: Color32::from_rgba_premultiplied(0x2E, 0x33, 0x3E, 0x90),
            grid_major_line: Color32::from_rgba_premultiplied(0x3E, 0x44, 0x52, 0xA0),
            guide_line: Color32::from_rgb(0x20, 0xD3, 0xB5),
            ruler_tick: Color32::from_rgb(0x5D, 0x65, 0x75),
            align_guide: Color32::from_rgb(0xCC, 0x79, 0xA7),
            polar_guide: Color32::from_rgb(0x56, 0xB4, 0xE9),
            selection_fill: Color32::from_rgba_premultiplied(0x7C, 0x5C, 0xFF, 0x28),
            selection_stroke: Color32::from_rgb(0x7C, 0x5C, 0xFF),
            page_border: Color32::from_rgb(0xB4, 0xB9, 0xC3),
            page_shadow: Color32::from_black_alpha(90),
            backdrop_scrim: Color32::from_black_alpha(140),
            hud_pill_bg: Color32::from_rgba_premultiplied(0x1C, 0x1F, 0x25, 0xEE),
            laser_cut: Color32::from_rgb(0xFF, 0x50, 0x50),
            laser_score: Color32::from_rgb(0x50, 0xA0, 0xFF),
            laser_active: Color32::from_rgb(0x0F, 0x93, 0x88),
            transparent: Color32::TRANSPARENT,
            snap_node: Color32::from_rgb(0xE6, 0x9F, 0x00),
            snap_center: Color32::from_rgb(0x56, 0xB4, 0xE9),
            snap_endpoint: Color32::from_rgb(0x00, 0xB8, 0x87),
            snap_midpoint: Color32::from_rgb(0xF0, 0xE4, 0x42),
            snap_quadrant: Color32::from_rgb(0x70, 0xAE, 0xEB),
            snap_intersection: Color32::from_rgb(0xF2, 0x7D, 0x3A),
            snap_grid: Color32::from_rgb(0x9A, 0xA1, 0xB2),
            snap_perpendicular: Color32::from_rgb(0xD8, 0x92, 0xBA),
            snap_tangent: Color32::from_rgb(0x88, 0xCC, 0xEE),
        }
    }

    /// Returns the light palette (WCAG AA compliant against `#F7F8FA` / `#FFFFFF`).
    pub const fn light() -> Self {
        Self {
            surface: Color32::from_rgb(0xF7, 0xF8, 0xFA),
            surface_elevated: Color32::from_rgb(0xFF, 0xFF, 0xFF),
            surface_interactive: Color32::from_rgb(0xEB, 0xEE, 0xF3),
            surface_hover: Color32::from_rgb(0xDF, 0xE4, 0xEC),
            surface_active: Color32::from_rgb(0xD2, 0xD9, 0xE5),
            canvas_bg: Color32::from_rgb(0xE5, 0xE8, 0xEE),
            page_bg: Color32::from_rgb(0xFF, 0xFF, 0xFF),
            on_surface: Color32::from_rgb(0x16, 0x19, 0x20),
            on_surface_muted: Color32::from_rgb(0x4B, 0x52, 0x62),
            on_surface_disabled: Color32::from_rgb(0x8D, 0x95, 0xA5),
            border: Color32::from_rgb(0xCB, 0xD2, 0xDE),
            border_focus: Color32::from_rgb(0x00, 0x72, 0xB2),
            accent: Color32::from_rgb(0x5B, 0x3D, 0xE8),
            accent_hover: Color32::from_rgb(0x47, 0x29, 0xCF),
            on_accent: Color32::from_rgb(0xFF, 0xFF, 0xFF),
            danger: Color32::from_rgb(0xB8, 0x28, 0x1B),
            warning: Color32::from_rgb(0x94, 0x5A, 0x00),
            success: Color32::from_rgb(0x00, 0x75, 0x55),
            grid_line: Color32::from_rgba_premultiplied(0xA8, 0xB1, 0xC2, 0x70),
            grid_major_line: Color32::from_rgba_premultiplied(0x85, 0x90, 0xA4, 0xA0),
            guide_line: Color32::from_rgb(0x00, 0x85, 0x73),
            ruler_tick: Color32::from_rgb(0x73, 0x7B, 0x8C),
            align_guide: Color32::from_rgb(0x9E, 0x3D, 0x76),
            polar_guide: Color32::from_rgb(0x00, 0x72, 0xB2),
            selection_fill: Color32::from_rgba_premultiplied(0x5B, 0x3D, 0xE8, 0x24),
            selection_stroke: Color32::from_rgb(0x5B, 0x3D, 0xE8),
            page_border: Color32::from_rgb(0x9A, 0xA2, 0xB0),
            page_shadow: Color32::from_black_alpha(45),
            backdrop_scrim: Color32::from_black_alpha(90),
            hud_pill_bg: Color32::from_rgba_premultiplied(0xF2, 0xF2, 0xF2, 0xF2),
            laser_cut: Color32::from_rgb(0xC8, 0x1E, 0x1E),
            laser_score: Color32::from_rgb(0x1D, 0x4E, 0xD8),
            laser_active: Color32::from_rgb(0x0A, 0x7B, 0x72),
            transparent: Color32::TRANSPARENT,
            snap_node: Color32::from_rgb(0x8F, 0x58, 0x00),
            snap_center: Color32::from_rgb(0x00, 0x68, 0xA3),
            snap_endpoint: Color32::from_rgb(0x00, 0x70, 0x51),
            snap_midpoint: Color32::from_rgb(0x75, 0x68, 0x00),
            snap_quadrant: Color32::from_rgb(0x00, 0x52, 0x8A),
            snap_intersection: Color32::from_rgb(0xA8, 0x3C, 0x00),
            snap_grid: Color32::from_rgb(0x52, 0x58, 0x66),
            snap_perpendicular: Color32::from_rgb(0x8F, 0x35, 0x6A),
            snap_tangent: Color32::from_rgb(0x00, 0x6D, 0x7A),
        }
    }

    /// Maps a [`SnapKind`] to its colorblind-safe Okabe–Ito token color.
    pub const fn snap_color(&self, kind: SnapKind) -> Color32 {
        match kind {
            SnapKind::Node => self.snap_node,
            SnapKind::Center => self.snap_center,
            SnapKind::Endpoint => self.snap_endpoint,
            SnapKind::Midpoint => self.snap_midpoint,
            SnapKind::Quadrant => self.snap_quadrant,
            SnapKind::Intersection => self.snap_intersection,
            SnapKind::Grid => self.snap_grid,
            SnapKind::Perpendicular => self.snap_perpendicular,
            SnapKind::Tangent => self.snap_tangent,
        }
    }

    /// Scales a token color's opacity by `alpha_factor` in `0.0..=1.0`.
    pub fn with_alpha(color: Color32, alpha_factor: f32) -> Color32 {
        let f = alpha_factor.clamp(0.0, 1.0);
        Color32::from_rgba_premultiplied(
            ((color.r() as f32) * f) as u8,
            ((color.g() as f32) * f) as u8,
            ((color.b() as f32) * f) as u8,
            ((color.a() as f32) * f) as u8,
        )
    }

    /// Replaces the alpha channel of an unmultiplied token color.
    pub fn with_alpha_u8(color: Color32, alpha: u8) -> Color32 {
        Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
    }

    /// Converts a document-domain [`core_doc::Color`] to an `egui::Color32`.
    pub fn from_doc_color(c: core_doc::Color) -> Color32 {
        Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a)
    }

    /// Converts an `egui::Color32` back to [`core_doc::Color`].
    pub fn to_doc_color(c: Color32) -> core_doc::Color {
        core_doc::Color::rgba(c.r(), c.g(), c.b(), c.a())
    }

    /// Converts a 3-byte sRGB array (e.g. from CAM layer palette) into `egui::Color32`.
    pub fn from_rgb_bytes(rgb: [u8; 3]) -> Color32 {
        Color32::from_rgb(rgb[0], rgb[1], rgb[2])
    }

    /// Converts an `egui::Color32` token into a `tiny_skia::Color`.
    pub fn to_skia_color(c: Color32) -> tiny_skia::Color {
        tiny_skia::Color::from_rgba8(c.r(), c.g(), c.b(), c.a())
    }

    /// Converts an `egui::Color32` token into a `tiny_skia::Color` with explicit alpha.
    pub fn to_skia_alpha(c: Color32, alpha: u8) -> tiny_skia::Color {
        tiny_skia::Color::from_rgba8(c.r(), c.g(), c.b(), alpha)
    }
}

/// Complete design token bundle shared by egui visuals, canvas overlays, and panels.
#[derive(Debug, Clone, PartialEq)]
pub struct ThemeTokens {
    pub is_dark: bool,
    pub spacing: SpacingTokens,
    pub radius: RadiusTokens,
    pub typography: TypographyTokens,
    pub color: SemanticColors,
}

impl ThemeTokens {
    /// Resolves token bundle directly for a known dark/light surface flag.
    pub const fn for_dark(is_dark: bool) -> Self {
        Self {
            is_dark,
            spacing: SpacingTokens,
            radius: RadiusTokens,
            typography: TypographyTokens,
            color: if is_dark {
                SemanticColors::dark()
            } else {
                SemanticColors::light()
            },
        }
    }

    /// Resolves token bundle for a persisted [`ThemePref`] (`System` resolves cleanly).
    pub fn for_pref(pref: ThemePref) -> Self {
        match pref.0 {
            ThemePreference::Dark => Self::for_dark(true),
            ThemePreference::Light => Self::for_dark(false),
            ThemePreference::System => Self::for_dark(detect_system_dark()),
        }
    }

    /// Resolves active [`ThemeTokens`] from `egui::Context` visuals.
    pub fn from_ctx(ctx: &egui::Context) -> Self {
        Self::for_dark(ctx.style().visuals.dark_mode)
    }

    /// Builds `egui::Visuals` driven cleanly by this token bundle (with visible focus rings).
    pub fn to_visuals(&self) -> Visuals {
        let mut v = if self.is_dark {
            Visuals::dark()
        } else {
            Visuals::light()
        };
        let c = &self.color;

        v.window_fill = c.surface_elevated;
        v.panel_fill = c.surface_elevated;
        v.faint_bg_color = c.surface;
        v.extreme_bg_color = c.canvas_bg;
        v.override_text_color = Some(c.on_surface);

        v.widgets.noninteractive.bg_fill = c.surface_elevated;
        v.widgets.noninteractive.weak_bg_fill = c.surface_elevated;
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, c.border);
        v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, c.on_surface_muted);
        v.widgets.noninteractive.corner_radius = CornerRadius::same(RadiusTokens::MD);

        v.widgets.inactive.bg_fill = c.surface_interactive;
        v.widgets.inactive.weak_bg_fill = c.surface_interactive;
        v.widgets.inactive.bg_stroke = Stroke::new(1.0, c.border);
        v.widgets.inactive.fg_stroke = Stroke::new(1.0, c.on_surface);
        v.widgets.inactive.corner_radius = CornerRadius::same(RadiusTokens::MD);

        v.widgets.hovered.bg_fill = c.surface_hover;
        v.widgets.hovered.weak_bg_fill = c.surface_hover;
        v.widgets.hovered.bg_stroke = Stroke::new(1.2, c.accent_hover);
        v.widgets.hovered.fg_stroke = Stroke::new(1.0, c.on_surface);
        v.widgets.hovered.corner_radius = CornerRadius::same(RadiusTokens::MD);

        v.widgets.active.bg_fill = c.surface_active;
        v.widgets.active.weak_bg_fill = c.surface_active;
        v.widgets.active.bg_stroke = Stroke::new(2.0, c.border_focus);
        v.widgets.active.fg_stroke = Stroke::new(1.5, c.on_surface);
        v.widgets.active.corner_radius = CornerRadius::same(RadiusTokens::MD);

        v.widgets.open.bg_fill = c.surface_active;
        v.widgets.open.weak_bg_fill = c.surface_active;
        v.widgets.open.bg_stroke = Stroke::new(1.5, c.border_focus);
        v.widgets.open.fg_stroke = Stroke::new(1.0, c.on_surface);
        v.widgets.open.corner_radius = CornerRadius::same(RadiusTokens::MD);

        v.selection.bg_fill = c.selection_fill;
        v.selection.stroke = Stroke::new(1.5, c.selection_stroke);
        v.hyperlink_color = c.accent_hover;
        v.window_corner_radius = CornerRadius::same(RadiusTokens::MD);
        v.window_stroke = Stroke::new(1.0, c.border);
        v.window_shadow = Shadow {
            offset: [0, 4],
            blur: 16,
            spread: 0,
            color: Color32::from_black_alpha(if self.is_dark { 120 } else { 40 }),
        };

        v
    }
}

/// Loads persisted [`ThemePref`] from `eframe::Storage` (defaults to `System` on first launch).
pub fn load_theme_pref(storage: Option<&dyn eframe::Storage>) -> ThemePref {
    let Some(s) = storage else {
        return ThemePref::default();
    };
    match s.get_string(STORAGE_THEME_KEY).as_deref() {
        Some("dark") => ThemePref(ThemePreference::Dark),
        Some("light") => ThemePref(ThemePreference::Light),
        _ => ThemePref(ThemePreference::System),
    }
}

/// Saves [`ThemePref`] into `eframe::Storage`.
pub fn save_theme_pref(storage: &mut dyn eframe::Storage, pref: ThemePref) {
    let val = match pref.0 {
        ThemePreference::Dark => "dark",
        ThemePreference::Light => "light",
        ThemePreference::System => "system",
    };
    storage.set_string(STORAGE_THEME_KEY, val.to_string());
}

fn detect_system_dark() -> bool {
    // Non-blocking fallback when resolved outside a live egui::Context frame.
    if let Ok(theme) = std::env::var("GTK_THEME") {
        if theme.to_ascii_lowercase().contains("dark") {
            return true;
        }
    }
    true
}

/// Computes WCAG 2.1 contrast ratio (`1.0 ..= 21.0`) between two opaque sRGB colors.
pub fn contrast_ratio(a: Color32, b: Color32) -> f64 {
    fn rel_luminance(c: Color32) -> f64 {
        fn linear(channel: u8) -> f64 {
            let s = (channel as f64) / 255.0;
            if s <= 0.04045 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        }
        0.2126 * linear(c.r()) + 0.7152 * linear(c.g()) + 0.0722 * linear(c.b())
    }

    let la = rel_luminance(a);
    let lb = rel_luminance(b);
    let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[derive(Default)]
    struct MockStorage(HashMap<String, String>);

    impl eframe::Storage for MockStorage {
        fn get_string(&self, key: &str) -> Option<String> {
            self.0.get(key).cloned()
        }

        fn set_string(&mut self, key: &str, value: String) {
            self.0.insert(key.to_string(), value);
        }

        fn flush(&mut self) {}
    }

    #[test]
    fn snap_colors_pass_wcag_aa_contrast_on_both_surfaces() {
        for dark in [true, false] {
            let t = ThemeTokens::for_dark(dark);
            let bg = t.color.surface;
            let checks = [
                ("on_surface", t.color.on_surface),
                ("on_surface_muted", t.color.on_surface_muted),
                ("snap_node", t.color.snap_node),
                ("snap_center", t.color.snap_center),
                ("snap_endpoint", t.color.snap_endpoint),
                ("snap_midpoint", t.color.snap_midpoint),
                ("snap_quadrant", t.color.snap_quadrant),
                ("snap_intersection", t.color.snap_intersection),
                ("snap_grid", t.color.snap_grid),
                ("snap_perpendicular", t.color.snap_perpendicular),
                ("snap_tangent", t.color.snap_tangent),
            ];
            for (name, fg) in checks {
                let ratio = contrast_ratio(fg, bg);
                assert!(
                    ratio >= 4.5,
                    "Contrast check failed for {name} (dark={dark}): {ratio:.2} < 4.5"
                );
            }
        }
    }

    #[test]
    fn theme_pref_roundtrips_through_storage() {
        let mut storage = MockStorage::default();
        assert_eq!(
            load_theme_pref(Some(&storage)),
            ThemePref(ThemePreference::System)
        );

        for pref in [
            ThemePref(ThemePreference::System),
            ThemePref(ThemePreference::Dark),
            ThemePref(ThemePreference::Light),
        ] {
            save_theme_pref(&mut storage, pref);
            assert_eq!(load_theme_pref(Some(&storage)), pref);
        }
    }
}
