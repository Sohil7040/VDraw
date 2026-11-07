//! HUD Toast notification overlay for instant visual feedback on actions.

use std::time::Instant;

/// Floating HUD toast notification manager.
pub struct ToastManager {
    message: Option<String>,
    timestamp: Option<Instant>,
    duration_secs: f32,
}

impl Default for ToastManager {
    fn default() -> Self {
        Self {
            message: None,
            timestamp: None,
            duration_secs: 3.0,
        }
    }
}

impl ToastManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Display a toast notification with the given message.
    pub fn show(&mut self, text: impl Into<String>) {
        self.message = Some(text.into());
        self.timestamp = Some(Instant::now());
    }

    /// Render the active toast notification if any.
    pub fn render(&mut self, ctx: &egui::Context) {
        let (Some(msg), Some(time)) = (&self.message, self.timestamp) else {
            return;
        };

        let elapsed = time.elapsed().as_secs_f32();
        if elapsed > self.duration_secs {
            self.message = None;
            self.timestamp = None;
            return;
        }

        // Smooth fade-in and fade-out
        let alpha = if elapsed > self.duration_secs - 0.4 {
            ((self.duration_secs - elapsed) / 0.4).clamp(0.0, 1.0)
        } else if elapsed < 0.15 {
            (elapsed / 0.15).clamp(0.0, 1.0)
        } else {
            1.0
        };

        let screen_rect = ctx.screen_rect();
        let toast_pos = egui::pos2(screen_rect.max.x - 24.0, screen_rect.max.y - 42.0);

        egui::Area::new(egui::Id::new("hud_toast_area"))
            .fixed_pos(toast_pos)
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(0.0, 0.0))
            .order(egui::Order::Tooltip)
            .interactable(false)
            .show(ctx, |ui| {
                let bg = egui::Color32::from_rgba_premultiplied(
                    (0x18 as f32 * alpha) as u8,
                    (0x1B as f32 * alpha) as u8,
                    (0x22 as f32 * alpha) as u8,
                    (245.0 * alpha) as u8,
                );
                let stroke = egui::Color32::from_rgba_premultiplied(
                    (0x7C as f32 * alpha) as u8,
                    (0x5C as f32 * alpha) as u8,
                    (0xFF as f32 * alpha) as u8,
                    (220.0 * alpha) as u8,
                );
                let text_color = egui::Color32::from_rgba_premultiplied(
                    (240.0 * alpha) as u8,
                    (245.0 * alpha) as u8,
                    (255.0 * alpha) as u8,
                    (255.0 * alpha) as u8,
                );

                egui::Frame::NONE
                    .fill(bg)
                    .stroke(egui::Stroke::new(1.0, stroke))
                    .corner_radius(egui::CornerRadius::same(10))
                    .shadow(egui::Shadow {
                        offset: [0, 4],
                        blur: 16,
                        spread: 0,
                        color: egui::Color32::from_black_alpha((140.0 * alpha) as u8),
                    })
                    .inner_margin(egui::Margin::symmetric(14, 8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("✦")
                                    .color(crate::theme::ACCENT)
                                    .strong(),
                            );
                            ui.label(egui::RichText::new(msg).color(text_color).strong());
                        });
                    });
            });

        ctx.request_repaint();
    }
}
