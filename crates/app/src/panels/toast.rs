//! HUD Toast notification overlay for non-blocking feedback and error details.

use crate::tokens::{RadiusTokens, ThemeTokens};
use std::time::Instant;

/// Severity level for a floating HUD toast.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToastLevel {
    #[default]
    Info,
    Success,
    Warning,
    Danger,
}

/// Floating HUD toast notification manager.
pub struct ToastManager {
    message: Option<String>,
    details: Option<String>,
    level: ToastLevel,
    timestamp: Option<Instant>,
    duration_secs: f32,
}

impl Default for ToastManager {
    fn default() -> Self {
        Self {
            message: None,
            details: None,
            level: ToastLevel::Info,
            timestamp: None,
            duration_secs: 3.0,
        }
    }
}

impl ToastManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Display a standard info/success toast notification.
    pub fn show(&mut self, text: impl Into<String>) {
        self.message = Some(text.into());
        self.details = None;
        self.level = ToastLevel::Info;
        self.timestamp = Some(Instant::now());
        self.duration_secs = 3.0;
    }

    /// Display a success toast notification.
    pub fn show_success(&mut self, text: impl Into<String>) {
        self.message = Some(text.into());
        self.details = None;
        self.level = ToastLevel::Success;
        self.timestamp = Some(Instant::now());
        self.duration_secs = 3.0;
    }

    /// Display a warning toast notification.
    pub fn show_warning(&mut self, text: impl Into<String>) {
        self.message = Some(text.into());
        self.details = None;
        self.level = ToastLevel::Warning;
        self.timestamp = Some(Instant::now());
        self.duration_secs = 5.0;
    }

    /// Display a non-blocking error toast with copyable failure details.
    pub fn show_error(&mut self, summary: impl Into<String>, details: impl Into<String>) {
        self.message = Some(summary.into());
        self.details = Some(details.into());
        self.level = ToastLevel::Danger;
        self.timestamp = Some(Instant::now());
        self.duration_secs = 8.0;
    }

    /// Returns the currently active toast summary, if any.
    pub fn current_message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    /// Dismisses the active toast immediately.
    pub fn dismiss(&mut self) {
        self.message = None;
        self.details = None;
        self.timestamp = None;
    }

    /// Render the active toast notification if any.
    pub fn render(&mut self, ctx: &egui::Context) {
        let (Some(msg), Some(time)) = (self.message.clone(), self.timestamp) else {
            return;
        };

        let elapsed = time.elapsed().as_secs_f32();
        if elapsed > self.duration_secs {
            self.dismiss();
            return;
        }

        let alpha = if elapsed > self.duration_secs - 0.4 {
            ((self.duration_secs - elapsed) / 0.4).clamp(0.0, 1.0)
        } else if elapsed < 0.15 {
            (elapsed / 0.15).clamp(0.0, 1.0)
        } else {
            1.0
        };

        let tokens = ThemeTokens::from_ctx(ctx);
        let c = &tokens.color;
        let accent_col = match self.level {
            ToastLevel::Info => c.accent,
            ToastLevel::Success => c.success,
            ToastLevel::Warning => c.warning,
            ToastLevel::Danger => c.danger,
        };
        let icon = match self.level {
            ToastLevel::Info => "✦",
            ToastLevel::Success => "✓",
            ToastLevel::Warning => "⚠",
            ToastLevel::Danger => "✕",
        };

        let bg = crate::tokens::SemanticColors::with_alpha(c.surface_elevated, 0.96 * alpha);
        let stroke = crate::tokens::SemanticColors::with_alpha(accent_col, 0.9 * alpha);
        let text_color = crate::tokens::SemanticColors::with_alpha(c.on_surface, alpha);
        let muted_color = crate::tokens::SemanticColors::with_alpha(c.on_surface_muted, alpha);
        let shadow_col = crate::tokens::SemanticColors::with_alpha(c.page_shadow, alpha);

        let screen_rect = ctx.screen_rect();
        let toast_pos = egui::pos2(screen_rect.max.x - 24.0, screen_rect.max.y - 42.0);
        let has_details = self.details.is_some();

        let mut dismiss_clicked = false;
        let mut copy_clicked = false;

        egui::Area::new(egui::Id::new("hud_toast_area"))
            .fixed_pos(toast_pos)
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(0.0, 0.0))
            .order(egui::Order::Tooltip)
            .interactable(has_details)
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(bg)
                    .stroke(egui::Stroke::new(1.5, stroke))
                    .corner_radius(egui::CornerRadius::same(RadiusTokens::LG))
                    .shadow(egui::Shadow {
                        offset: [0, 4],
                        blur: 16,
                        spread: 0,
                        color: shadow_col,
                    })
                    .inner_margin(egui::Margin::symmetric(14, 10))
                    .show(ui, |ui| {
                        ui.set_max_width(420.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(icon).color(accent_col).strong());
                            ui.label(egui::RichText::new(&msg).color(text_color).strong());
                            if has_details {
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui.small_button("✕").clicked() {
                                            dismiss_clicked = true;
                                        }
                                        if ui
                                            .small_button("Copy details")
                                            .on_hover_text("Copy error details to clipboard")
                                            .clicked()
                                        {
                                            copy_clicked = true;
                                        }
                                    },
                                );
                            }
                        });
                        if let Some(details) = &self.details {
                            if !details.is_empty() && details != &msg {
                                ui.add_space(2.0);
                                ui.label(egui::RichText::new(details).small().color(muted_color));
                            }
                        }
                    });
            });

        if copy_clicked {
            if let Some(details) = &self.details {
                ctx.copy_text(format!("{msg}\n{details}"));
            } else {
                ctx.copy_text(msg);
            }
        }
        if dismiss_clicked {
            self.dismiss();
        } else {
            ctx.request_repaint();
        }
    }
}
