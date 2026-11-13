//! Theme layout tokens and visuals builder delegating colors exclusively to [`crate::tokens::ThemeTokens`].
#![allow(dead_code)]

use crate::tokens::{RadiusTokens, SpacingTokens, ThemeTokens};

// ─── Spacing & sizing tokens ─────────────────────────────────────────────────

pub const SPACING: f32 = SpacingTokens::SM;
pub const CORNER_RADIUS: u8 = RadiusTokens::MD;
pub const TOOLBAR_WIDTH: f32 = SpacingTokens::TOOLBAR_WIDTH;
pub const RULER_SIZE: f32 = SpacingTokens::RULER_SIZE;
pub const STATUSBAR_HEIGHT: f32 = SpacingTokens::STATUSBAR_HEIGHT;
pub const PROPBAR_HEIGHT: f32 = SpacingTokens::COMMANDBAR_HEIGHT;

// ─── Visuals builder ─────────────────────────────────────────────────────────

/// Build an `egui::Visuals` using VDraw's semantic design tokens.
pub fn build_visuals(dark: bool) -> egui::Visuals {
    ThemeTokens::for_dark(dark).to_visuals()
}

/// Apply VDraw's spacing style to an `egui::Style`.
pub fn apply_style(style: &mut egui::Style) {
    style.spacing.item_spacing = egui::vec2(SPACING, SPACING * 0.5);
    style.spacing.button_padding = egui::vec2(SPACING, SPACING * 0.5);
    style.spacing.indent = SPACING * 2.0;
    style.spacing.scroll.bar_width = 6.0;
}
