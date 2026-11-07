//! Dock panel for Phase 4 Boolean CAD Operations, Parallel Contours, and Vector Text.

use crate::panels::toast::ToastManager;
use crate::theme;
use core_boolean::{
    apply_boolean_to_document, apply_contour_to_document, apply_text_to_document, BooleanOp,
    ContourJoin,
};
use core_doc::{Document, UndoStack};
use core_geom::Point2;

/// Persistent state for the Boolean and Contour dock tab.
#[derive(Debug)]
pub struct BooleanPanelState {
    pub offset_mm: f64,
    pub join_type: ContourJoin,
    pub text_input: String,
    pub font_size_mm: f64,
}

impl Default for BooleanPanelState {
    fn default() -> Self {
        Self {
            offset_mm: 5.0,
            join_type: ContourJoin::Round,
            text_input: "VDraw Laser".to_string(),
            font_size_mm: 20.0,
        }
    }
}

/// Show the Boolean Shaping and CAM Contour tools in the dock panel.
pub fn show_boolean_panel(
    ui: &mut egui::Ui,
    state: &mut BooleanPanelState,
    doc: &mut Document,
    undo: &mut UndoStack,
    toast: &mut ToastManager,
) {
    ui.heading("Boolean Shaping");
    ui.separator();

    let sel_count = doc.selection.len();

    // ── 1. Selection Status Badge ──────────────────────────────────────────
    ui.horizontal(|ui| {
        let badge_color = if sel_count >= 2 {
            theme::ACCENT
        } else if sel_count == 1 {
            theme::DYNAMIC_GUIDE
        } else {
            theme::TEXT_DISABLED
        };

        ui.label(
            egui::RichText::new(format!("● {} Shape(s) Selected", sel_count))
                .color(badge_color)
                .small()
                .strong(),
        );
    });

    ui.add_space(6.0);

    // ── 2. Boolean Operations Grid ────────────────────────────────────────
    let can_boolean = sel_count >= 2;
    let ops = [
        (
            BooleanOp::Weld,
            "⊞ Weld",
            "Combine overlapping shapes into a single unified outline",
        ),
        (
            BooleanOp::Trim,
            "⊟ Trim",
            "Subtract foreground shapes from the background shape",
        ),
        (
            BooleanOp::Intersect,
            "⊠ Intersect",
            "Retain only the overlapping area common to all shapes",
        ),
        (
            BooleanOp::Exclude,
            "⊞̷ Exclude",
            "XOR: Retain only areas where an odd number of shapes overlap",
        ),
        (
            BooleanOp::FrontMinusBack,
            "⬒ Front - Back",
            "Subtract background shape from foreground shape",
        ),
        (
            BooleanOp::BackMinusFront,
            "⬓ Back - Front",
            "Subtract foreground shape from background shape",
        ),
        (
            BooleanOp::CreateBoundary,
            "⬚ Boundary",
            "Create a new closed perimeter around all selected shapes",
        ),
        (
            BooleanOp::Simplify,
            "◌ Simplify",
            "Remove redundant collinear vertices and normalize paths",
        ),
    ];

    ui.group(|ui| {
        ui.label(
            egui::RichText::new("COMBINE & SHAPING")
                .size(10.5)
                .color(theme::TEXT_SECONDARY)
                .strong(),
        );
        ui.add_space(4.0);

        for (op, label, tooltip) in ops {
            let enabled = if matches!(op, BooleanOp::Simplify) {
                sel_count >= 1
            } else {
                can_boolean
            };

            let btn = egui::Button::new(label).min_size(egui::vec2(ui.available_width(), 26.0));

            let r = ui.add_enabled(enabled, btn).on_hover_text(tooltip);
            if r.clicked() {
                match apply_boolean_to_document(doc, undo, op, true) {
                    Ok(msg) => {
                        toast.show(format!("✓ {}", msg));
                    }
                    Err(e) => {
                        toast.show(format!("⚠ Boolean failed: {}", e));
                    }
                }
            }
            ui.add_space(2.0);
        }
    });

    ui.add_space(10.0);

    // ── 3. Parallel Contour / Offset Section ──────────────────────────────
    ui.group(|ui| {
        ui.label(
            egui::RichText::new("CONTOUR / PARALLEL OFFSET")
                .size(10.5)
                .color(theme::TEXT_SECONDARY)
                .strong(),
        );
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("Offset:");
            ui.add(
                egui::DragValue::new(&mut state.offset_mm)
                    .speed(0.5)
                    .range(-100.0..=100.0)
                    .suffix(" mm"),
            );
        });

        ui.horizontal(|ui| {
            ui.label("Corner:");
            ui.selectable_value(&mut state.join_type, ContourJoin::Round, "Round");
            ui.selectable_value(&mut state.join_type, ContourJoin::Miter(2.0), "Miter");
            ui.selectable_value(&mut state.join_type, ContourJoin::Bevel, "Bevel");
        });

        ui.add_space(4.0);
        let btn = egui::Button::new("⤢ Apply Contour Offset")
            .min_size(egui::vec2(ui.available_width(), 26.0));
        let r = ui.add_enabled(sel_count >= 1, btn).on_hover_text(
            "Generate parallel offset contours for laser kerf compensation or decorative outlines",
        );

        if r.clicked() {
            match apply_contour_to_document(doc, undo, state.offset_mm, state.join_type) {
                Ok(msg) => {
                    toast.show(format!("✓ {}", msg));
                }
                Err(e) => {
                    toast.show(format!("⚠ Contour error: {}", e));
                }
            }
        }
    });

    ui.add_space(10.0);

    // ── 4. Vector Text to Outlines Section ────────────────────────────────
    ui.group(|ui| {
        ui.label(
            egui::RichText::new("VECTOR TEXT OUTLINES")
                .size(10.5)
                .color(theme::TEXT_SECONDARY)
                .strong(),
        );
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("Text:");
            ui.text_edit_singleline(&mut state.text_input);
        });

        ui.horizontal(|ui| {
            ui.label("Size:");
            ui.add(
                egui::DragValue::new(&mut state.font_size_mm)
                    .speed(1.0)
                    .range(4.0..=200.0)
                    .suffix(" mm"),
            );
        });

        ui.add_space(4.0);
        let btn = egui::Button::new("T Insert Text Outline")
            .min_size(egui::vec2(ui.available_width(), 26.0));

        if ui.add(btn).clicked() {
            let center = if let Some(bounds) = doc.selection_bounds() {
                Point2::new(bounds.center().x, bounds.center().y + 30.0)
            } else if let Some(p) = doc.active_page() {
                Point2::new(p.width_mm * 0.3, p.height_mm * 0.5)
            } else {
                Point2::new(50.0, 50.0)
            };

            match apply_text_to_document(doc, undo, &state.text_input, state.font_size_mm, center) {
                Ok(msg) => {
                    toast.show(format!("✓ {}", msg));
                }
                Err(e) => {
                    toast.show(format!("⚠ Text rendering error: {}", e));
                }
            }
        }
    });
}
