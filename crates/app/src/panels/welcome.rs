//! Startup Welcome Dialog and 3-step First-Run Coach Marks.

use crate::panels::toast::ToastManager;
use crate::tokens::{RadiusTokens, ThemeTokens};
use core_doc::Document;

const STORAGE_COACH_SEEN_KEY: &str = "vdraw.onboarding.coach_seen";

/// Loads whether the first-run coach marks tour has been completed/dismissed.
pub fn load_coach_seen(storage: Option<&dyn eframe::Storage>) -> bool {
    let Some(s) = storage else {
        return false;
    };
    matches!(
        s.get_string(STORAGE_COACH_SEEN_KEY).as_deref(),
        Some("true")
    )
}

/// Persists the coach-marks seen flag into `eframe::Storage`.
pub fn save_coach_seen(storage: &mut dyn eframe::Storage, seen: bool) {
    storage.set_string(
        STORAGE_COACH_SEEN_KEY,
        if seen { "true" } else { "false" }.to_string(),
    );
}

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

/// Coach mark step metadata.
pub struct CoachStep {
    pub title: &'static str,
    pub body: &'static str,
    pub anchor_hint: &'static str,
}

pub const COACH_STEPS: &[CoachStep] = &[
    CoachStep {
        title: "Step 1 of 3 · Pick Tools & Command Palette",
        body: "Select drawing and editing tools from the left tool palette, or press Ctrl+K / use the top search bar to jump to any command.",
        anchor_hint: "Left Tool Palette & Top Search",
    },
    CoachStep {
        title: "Step 2 of 3 · Precision Snapping & Live HUD",
        body: "Hover or drag shapes to see colorblind-safe Okabe-Ito snap indicators. Toggle snap modes and HandleMode in the bottom status bar.",
        anchor_hint: "Central Canvas & Bottom Status Bar",
    },
    CoachStep {
        title: "Step 3 of 3 · Transform, Booleans & Laser CAM",
        body: "Use the right inspector for exact Rotate, Mirror, Scale, Boolean shaping, and Laser CAM toolpath setup.",
        anchor_hint: "Right Properties Inspector",
    },
];

/// State for the Welcome dialog and 3-step onboarding coach marks.
pub struct WelcomeState {
    pub is_open: bool,
    pub show_on_startup: bool,
    pub custom_width: f64,
    pub custom_height: f64,
    pub coach_active: bool,
    pub coach_step: usize,
    pub coach_seen: bool,
    logo_texture: Option<egui::TextureHandle>,
}

impl Default for WelcomeState {
    fn default() -> Self {
        Self {
            is_open: true,
            show_on_startup: true,
            custom_width: 210.0,
            custom_height: 297.0,
            coach_active: false,
            coach_step: 0,
            coach_seen: false,
            logo_texture: None,
        }
    }
}

impl WelcomeState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a quiet welcome state for headless / automated sessions.
    pub fn headless() -> Self {
        Self {
            is_open: false,
            coach_active: false,
            coach_seen: true,
            ..Self::default()
        }
    }

    /// Starts or restarts the 3-step coach marks tour.
    pub fn start_coach_marks(&mut self) {
        self.is_open = false;
        self.coach_active = true;
        self.coach_step = 0;
    }

    /// Renders the startup modal and/or active 3-step coach mark tour.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        doc: &mut Document,
        canvas: &mut crate::canvas::CanvasState,
        toast: &mut ToastManager,
    ) {
        self.show_coach_marks(ctx);

        if !self.is_open {
            return;
        }

        let tokens = ThemeTokens::from_ctx(ctx);
        let c = &tokens.color;

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.is_open = false;
        }

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
        let modal_width = 720.0f32.min(screen_rect.width() - 40.0).max(280.0);

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
                    .fill(c.surface_elevated)
                    .stroke(egui::Stroke::new(1.0, c.border_focus))
                    .corner_radius(egui::CornerRadius::same(RadiusTokens::LG))
                    .shadow(egui::Shadow {
                        offset: [0, 8],
                        blur: 32,
                        spread: 0,
                        color: c.page_shadow,
                    })
                    .inner_margin(egui::Margin::symmetric(24, 20)),
            )
            .show(ctx, |ui| {
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
                                    .color(c.on_surface)
                                    .strong(),
                            );
                            ui.label(
                                egui::RichText::new("v0.1.0")
                                    .size(11.0)
                                    .color(c.accent)
                                    .strong(),
                            );
                        });
                        ui.label(
                            egui::RichText::new("Precision 2D Vector CAD & Laser-CAM Suite")
                                .size(12.0)
                                .color(c.on_surface_muted),
                        );
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(
                                egui::RichText::new("✕")
                                    .size(14.0)
                                    .color(c.on_surface_muted),
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

                ui.label(
                    egui::RichText::new("START A NEW PROJECT")
                        .size(11.0)
                        .color(c.on_surface_muted)
                        .strong(),
                );
                ui.add_space(8.0);

                let card_width = (modal_width - 64.0) / 2.0;
                ui.columns(2, |cols| {
                    for (i, preset) in PRESETS.iter().enumerate() {
                        let col = &mut cols[i % 2];
                        let frame = egui::Frame::NONE
                            .fill(c.surface_interactive)
                            .stroke(egui::Stroke::new(1.0, c.border))
                            .corner_radius(egui::CornerRadius::same(RadiusTokens::MD))
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
                                                .color(c.on_surface)
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
                                                    .color(c.accent)
                                                    .monospace(),
                                                );
                                            },
                                        );
                                    });
                                    ui.label(
                                        egui::RichText::new(preset.description)
                                            .size(10.5)
                                            .color(c.on_surface_muted),
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
                                "Created {} ({:.0} × {:.0} mm)",
                                preset.name, preset.width_mm, preset.height_mm
                            ));
                            self.is_open = false;
                            if !self.coach_seen {
                                self.start_coach_marks();
                            }
                        }
                        col.add_space(6.0);
                    }
                });

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Custom Size:")
                            .small()
                            .color(c.on_surface_muted),
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
                            "Created Custom Page ({:.0} × {:.0} mm)",
                            self.custom_width, self.custom_height
                        ));
                        self.is_open = false;
                        if !self.coach_seen {
                            self.start_coach_marks();
                        }
                    }
                });

                ui.add_space(14.0);
                ui.separator();
                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.show_on_startup, "Show welcome screen on startup");

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let start_btn = egui::Button::new(
                            egui::RichText::new("Start Blank Canvas")
                                .color(c.on_accent)
                                .strong(),
                        )
                        .fill(c.accent);
                        if ui.add(start_btn).clicked() {
                            toast.show("Welcome to VDraw! Canvas ready.");
                            self.is_open = false;
                            if !self.coach_seen {
                                self.start_coach_marks();
                            }
                        }
                        if ui.button("Guided Tour (3 steps)").clicked() {
                            self.start_coach_marks();
                        }
                    });
                });
            });

        self.is_open = open;
    }

    fn show_coach_marks(&mut self, ctx: &egui::Context) {
        if !self.coach_active {
            return;
        }
        let step_idx = self.coach_step.min(COACH_STEPS.len().saturating_sub(1));
        let step = &COACH_STEPS[step_idx];
        let tokens = ThemeTokens::from_ctx(ctx);
        let c = &tokens.color;
        let screen_rect = ctx.screen_rect();

        let anchor_pos = match step_idx {
            0 => egui::pos2(screen_rect.min.x + 150.0, screen_rect.min.y + 84.0),
            1 => egui::pos2(screen_rect.center().x - 170.0, screen_rect.max.y - 190.0),
            _ => egui::pos2(screen_rect.max.x - 360.0, screen_rect.min.y + 120.0),
        };

        egui::Area::new(egui::Id::new("coach_marks_overlay"))
            .fixed_pos(anchor_pos)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(c.surface_elevated)
                    .stroke(egui::Stroke::new(1.5, c.border_focus))
                    .corner_radius(egui::CornerRadius::same(RadiusTokens::LG))
                    .inner_margin(egui::Margin::same(14))
                    .show(ui, |ui| {
                        ui.set_max_width(320.0);
                        ui.label(
                            egui::RichText::new(step.anchor_hint)
                                .small()
                                .color(c.accent_hover)
                                .strong(),
                        );
                        ui.add_space(2.0);
                        ui.label(egui::RichText::new(step.title).color(c.on_surface).strong());
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(step.body)
                                .small()
                                .color(c.on_surface_muted),
                        );
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            if ui.button("Skip Tour").clicked() {
                                self.coach_active = false;
                                self.coach_seen = true;
                            }
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let is_last = step_idx + 1 >= COACH_STEPS.len();
                                    let next_lbl = if is_last { "Finish" } else { "Next →" };
                                    let next_btn = egui::Button::new(
                                        egui::RichText::new(next_lbl).color(c.on_accent).strong(),
                                    )
                                    .fill(c.accent);
                                    if ui.add(next_btn).clicked() {
                                        if is_last {
                                            self.coach_active = false;
                                            self.coach_seen = true;
                                        } else {
                                            self.coach_step = step_idx + 1;
                                        }
                                    }
                                },
                            );
                        });
                    });
            });
    }
}
