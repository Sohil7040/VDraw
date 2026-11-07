//! Startup Welcome Dialog with document presets, branding, and quick start guides.

use crate::panels::toast::ToastManager;
use crate::theme;
use core_doc::Document;

/// Document template preset definition.
#[derive(Debug, Clone, Copy)]
pub struct TemplatePreset {
    pub name: &'static str,
    pub icon: &'static str,
    pub width_mm: f64,
    pub height_mm: f64,
    pub description: &'static str,
}

pub const PRESETS: &[TemplatePreset] = &[
    TemplatePreset {
        name: "Laser Sheet A4",
        icon: "📄",
        width_mm: 210.0,
        height_mm: 297.0,
        description: "Standard material sheet for test cuts, signage & prototyping",
    },
    TemplatePreset {
        name: "Laser Bed 600×400",
        icon: "⚡",
        width_mm: 600.0,
        height_mm: 400.0,
        description: "Medium-format CO2 & diode laser work area",
    },
    TemplatePreset {
        name: "Laser Bed 900×600",
        icon: "🏭",
        width_mm: 900.0,
        height_mm: 600.0,
        description: "Large industrial flatbed laser cutter workspace",
    },
    TemplatePreset {
        name: "CAD Technical Square",
        icon: "📐",
        width_mm: 300.0,
        height_mm: 300.0,
        description: "Precision 1:1 engineering drawing canvas",
    },
];

/// State for the Welcome dialog.
pub struct WelcomeState {
    pub is_open: bool,
    pub show_on_startup: bool,
    pub custom_width: f64,
    pub custom_height: f64,
    logo_texture: Option<egui::TextureHandle>,
}

impl Default for WelcomeState {
    fn default() -> Self {
        Self {
            is_open: true,
            show_on_startup: true,
            custom_width: 210.0,
            custom_height: 297.0,
            logo_texture: None,
        }
    }
}

impl WelcomeState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Renders the sleek startup modal if open.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        doc: &mut Document,
        canvas: &mut crate::canvas::CanvasState,
        toast: &mut ToastManager,
    ) {
        if !self.is_open {
            return;
        }

        // Close on Escape
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.is_open = false;
        }

        // Lazy load logo texture
        if self.logo_texture.is_none() {
            let png_bytes = include_bytes!("../../../../assets/icons/icon.png");
            if let Ok(pixmap) = tiny_skia::Pixmap::decode_png(png_bytes) {
                let color_image = egui::ColorImage::from_rgba_unmultiplied(
                    [pixmap.width() as usize, pixmap.height() as usize],
                    pixmap.data(),
                );
                self.logo_texture = Some(ctx.load_texture(
                    "vdraw_welcome_logo",
                    color_image,
                    egui::TextureOptions::LINEAR,
                ));
            }
        }

        let mut open = self.is_open;
        let screen_rect = ctx.screen_rect();
        let modal_width = 720.0f32.min(screen_rect.width() - 40.0);

        egui::Window::new("Welcome to VDraw")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .movable(true)
            .title_bar(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .default_width(modal_width)
            .max_width(modal_width)
            .frame(
                egui::Frame::NONE
                    .fill(theme::BG_PANEL)
                    .stroke(egui::Stroke::new(1.0, theme::BORDER_FOCUS))
                    .corner_radius(egui::CornerRadius::same(12))
                    .shadow(egui::Shadow {
                        offset: [0, 8],
                        blur: 32,
                        spread: 0,
                        color: egui::Color32::from_black_alpha(180),
                    })
                    .inner_margin(egui::Margin::symmetric(24, 20)),
            )
            .show(ctx, |ui| {
                // ── Header with Logo & Brand Title ──────────────────────────
                ui.horizontal(|ui| {
                    if let Some(texture) = &self.logo_texture {
                        ui.add(
                            egui::Image::from_texture(texture)
                                .fit_to_exact_size(egui::vec2(52.0, 52.0)),
                        );
                    }

                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("VDraw")
                                    .size(24.0)
                                    .color(theme::TEXT_PRIMARY)
                                    .strong(),
                            );
                            ui.label(
                                egui::RichText::new("v0.1.0")
                                    .size(11.0)
                                    .color(theme::ACCENT)
                                    .strong(),
                            );
                        });
                        ui.label(
                            egui::RichText::new("Precision 2D Vector CAD & Laser-CAM Suite")
                                .size(12.0)
                                .color(theme::TEXT_SECONDARY),
                        );
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(
                                egui::RichText::new("✕")
                                    .size(14.0)
                                    .color(theme::TEXT_SECONDARY),
                            )
                            .clicked()
                        {
                            self.is_open = false;
                        }
                    });
                });

                ui.add_space(14.0);
                ui.separator();
                ui.add_space(10.0);

                // ── Quick Presets Section ──────────────────────────────────
                ui.label(
                    egui::RichText::new("START A NEW PROJECT")
                        .size(11.0)
                        .color(theme::TEXT_SECONDARY)
                        .strong(),
                );
                ui.add_space(8.0);

                let card_width = (modal_width - 64.0) / 2.0;
                ui.columns(2, |cols| {
                    for (i, preset) in PRESETS.iter().enumerate() {
                        let col = &mut cols[i % 2];
                        let is_card_hovered = {
                            let frame = egui::Frame::NONE
                                .fill(theme::BG_WIDGET)
                                .stroke(egui::Stroke::new(1.0, theme::BORDER))
                                .corner_radius(egui::CornerRadius::same(8))
                                .inner_margin(egui::Margin::symmetric(12, 10));

                            let r = frame.show(col, |ui| {
                                ui.set_width(card_width);
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(preset.icon).size(22.0));
                                    ui.add_space(4.0);
                                    ui.vertical(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new(preset.name)
                                                    .color(theme::TEXT_PRIMARY)
                                                    .strong(),
                                            );
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    ui.label(
                                                        egui::RichText::new(format!(
                                                            "{:.0}×{:.0} mm",
                                                            preset.width_mm, preset.height_mm
                                                        ))
                                                        .size(10.5)
                                                        .color(theme::ACCENT)
                                                        .monospace(),
                                                    );
                                                },
                                            );
                                        });
                                        ui.label(
                                            egui::RichText::new(preset.description)
                                                .size(10.5)
                                                .color(theme::TEXT_SECONDARY),
                                        );
                                    });
                                });
                            });

                            let response = col.interact(
                                r.response.rect,
                                col.id().with(preset.name),
                                egui::Sense::click(),
                            );

                            if response.hovered() {
                                col.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }

                            if response.clicked() {
                                if let Some(p) = doc.active_page_mut() {
                                    p.width_mm = preset.width_mm;
                                    p.height_mm = preset.height_mm;
                                }
                                canvas.fit_page(screen_rect);
                                toast.show(format!(
                                    "✓ Created {} ({:.0} × {:.0} mm)",
                                    preset.name, preset.width_mm, preset.height_mm
                                ));
                                self.is_open = false;
                            }
                            response.hovered()
                        };
                        let _ = is_card_hovered;
                        col.add_space(6.0);
                    }
                });

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(8.0);

                // ── Keyboard Shortcuts Quick Reference ────────────────────
                ui.label(
                    egui::RichText::new("PRECISION TOOLS & SHORTCUTS")
                        .size(11.0)
                        .color(theme::TEXT_SECONDARY)
                        .strong(),
                );
                ui.add_space(6.0);

                ui.horizontal_wrapped(|ui| {
                    let shortcuts = [
                        ("V", "Pick"),
                        ("F10", "Node Tool"),
                        ("B", "Pen"),
                        ("F6", "Rectangle"),
                        ("F7", "Ellipse"),
                        ("K", "Knife Tool"),
                        ("X", "Eraser"),
                        ("✂ D", "Virtual Trim"),
                        ("T", "Text Outlines"),
                        ("0", "Fit Page"),
                        ("Ctrl+K", "Command Palette"),
                    ];

                    for (k, desc) in shortcuts {
                        let frame = egui::Frame::NONE
                            .fill(theme::BG_BASE)
                            .stroke(egui::Stroke::new(1.0, theme::BORDER))
                            .corner_radius(egui::CornerRadius::same(4))
                            .inner_margin(egui::Margin::symmetric(6, 3));

                        frame.show(ui, |ui| {
                            ui.label(egui::RichText::new(k).color(theme::ACCENT).strong().small());
                            ui.label(
                                egui::RichText::new(desc)
                                    .color(theme::TEXT_SECONDARY)
                                    .small(),
                            );
                        });
                        ui.add_space(4.0);
                    }
                });

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Custom Size:")
                            .small()
                            .color(theme::TEXT_SECONDARY),
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.custom_width)
                            .speed(1.0)
                            .range(10.0..=3000.0)
                            .suffix(" mm W"),
                    );
                    ui.label("×");
                    ui.add(
                        egui::DragValue::new(&mut self.custom_height)
                            .speed(1.0)
                            .range(10.0..=3000.0)
                            .suffix(" mm H"),
                    );
                    if ui.button("Create Custom").clicked() {
                        if let Some(p) = doc.active_page_mut() {
                            p.width_mm = self.custom_width;
                            p.height_mm = self.custom_height;
                        }
                        canvas.fit_page(screen_rect);
                        toast.show(format!(
                            "✓ Created Custom Page ({:.0} × {:.0} mm)",
                            self.custom_width, self.custom_height
                        ));
                        self.is_open = false;
                    }
                });

                ui.add_space(14.0);
                ui.separator();
                ui.add_space(10.0);

                // ── Footer controls ────────────────────────────────────────
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.show_on_startup, "Show welcome screen on startup");

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(
                                egui::RichText::new("Start Blank Canvas")
                                    .color(egui::Color32::WHITE)
                                    .strong(),
                            )
                            .clicked()
                        {
                            toast.show("✓ Welcome to VDraw! Canvas ready.");
                            self.is_open = false;
                        }
                    });
                });
            });

        self.is_open = open;
    }
}
