//! Document validation subsystem for verifying integrity, geometry soundness,
//! and schema validity on document load and prior to manufacturing CAM export.

use std::collections::HashSet;

use crate::document::Document;
use crate::shape::{Shape, ShapeId, ShapeKind};
use core_geom::rect::Rect;

/// Maximum supported coordinate range in millimeters (±100 meters).
pub const MAX_VALID_COORDINATE_MM: f64 = 100_000.0;

/// Current document schema version.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Specific validation failure encountered during document inspection.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ValidationError {
    #[error("Duplicate ShapeId detected: {0}")]
    DuplicateShapeId(ShapeId),

    #[error("Non-finite coordinate (NaN or Infinity) found in shape {0} ({1})")]
    NonFiniteCoordinate(ShapeId, String),

    #[error("Coordinate {1} mm in shape {0} exceeds supported range (±{2} mm)")]
    CoordinateOutOfBounds(ShapeId, f64, f64),

    #[error("Invalid page dimensions for page '{0}': {1} x {2} mm (must be positive and finite)")]
    InvalidPageDimensions(String, f64, f64),

    #[error("Empty group detected in shape {0}")]
    EmptyGroup(ShapeId),

    #[error("Recursive or broken group relationship detected in shape {0}")]
    BrokenGroup(ShapeId),

    #[error("Invalid geometry in shape {0}: {1}")]
    InvalidGeometry(ShapeId, String),

    #[error("Unsupported schema version: {0} (supported up to {1})")]
    UnsupportedSchemaVersion(u32, u32),

    #[error("Invalid guideline coordinate ({0} mm) in guideline #{1}")]
    InvalidGuideline(f64, u64),
}

/// Detailed validation report containing all detected errors and non-critical warnings.
#[derive(Debug, Clone, Default)]
pub struct ValidationReport {
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<String>,
}

impl ValidationReport {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Validates the complete integrity of a `Document`.
pub fn validate_document(doc: &Document) -> ValidationReport {
    let mut report = ValidationReport::default();
    let mut seen_ids = HashSet::new();

    // 1. Schema version check
    if doc.metadata.schema_version > CURRENT_SCHEMA_VERSION {
        report
            .errors
            .push(ValidationError::UnsupportedSchemaVersion(
                doc.metadata.schema_version,
                CURRENT_SCHEMA_VERSION,
            ));
    }

    // 2. Validate pages
    if doc.pages.is_empty() {
        report
            .warnings
            .push("Document contains zero pages".to_string());
    }

    for (p_idx, page) in doc.pages.iter().enumerate() {
        if !page.width_mm.is_finite()
            || !page.height_mm.is_finite()
            || page.width_mm <= 0.0
            || page.height_mm <= 0.0
        {
            report.errors.push(ValidationError::InvalidPageDimensions(
                page.name.clone(),
                page.width_mm,
                page.height_mm,
            ));
        }

        if page.layers.is_empty() {
            report
                .warnings
                .push(format!("Page {} ('{}') has no layers", p_idx, page.name));
        }

        // 3. Validate layers and shapes
        for layer in &page.layers {
            for shape in &layer.shapes {
                validate_shape_recursive(shape, &mut seen_ids, &mut report, 0);
            }
        }
    }

    // 4. Validate guidelines
    for g in &doc.guidelines {
        let pos = match g.orientation {
            crate::guideline::GuidelineOrientation::Horizontal(y) => y,
            crate::guideline::GuidelineOrientation::Vertical(x) => x,
            crate::guideline::GuidelineOrientation::Angled { .. } => 0.0,
        };
        if !pos.is_finite() || pos.abs() > MAX_VALID_COORDINATE_MM {
            report
                .errors
                .push(ValidationError::InvalidGuideline(pos, g.id.0));
        }
    }

    report
}

fn validate_shape_recursive(
    shape: &Shape,
    seen_ids: &mut HashSet<ShapeId>,
    report: &mut ValidationReport,
    depth: usize,
) {
    const MAX_GROUP_DEPTH: usize = 32;

    // Check duplicate IDs
    if !seen_ids.insert(shape.id) {
        report
            .errors
            .push(ValidationError::DuplicateShapeId(shape.id));
    }

    // Check transform finiteness
    if !shape.transform.is_finite() {
        report.errors.push(ValidationError::NonFiniteCoordinate(
            shape.id,
            "Affine2 transform contains NaN or Infinity".to_string(),
        ));
    }

    // Check depth
    if depth > MAX_GROUP_DEPTH {
        report.errors.push(ValidationError::BrokenGroup(shape.id));
        return;
    }

    // Validate geometry specifics
    match &shape.kind {
        ShapeKind::Path(poly) => {
            validate_polycurve(shape.id, poly, report);
        }
        ShapeKind::Rect {
            rect,
            corner_radius,
            ..
        } => {
            validate_rect(shape.id, rect, *corner_radius, report);
        }
        ShapeKind::Ellipse { center, rx, ry } => {
            if !center.is_finite() || !rx.is_finite() || !ry.is_finite() {
                report.errors.push(ValidationError::NonFiniteCoordinate(
                    shape.id,
                    "Ellipse center or radii are non-finite".to_string(),
                ));
            } else if *rx <= 0.0 || *ry <= 0.0 {
                report.errors.push(ValidationError::InvalidGeometry(
                    shape.id,
                    format!("Ellipse radii must be positive, got rx={}, ry={}", rx, ry),
                ));
            }
        }
        ShapeKind::Line { start, end } => {
            if !start.is_finite() || !end.is_finite() {
                report.errors.push(ValidationError::NonFiniteCoordinate(
                    shape.id,
                    "Line endpoints are non-finite".to_string(),
                ));
            } else if start.distance(*end) < 1e-9 {
                report
                    .warnings
                    .push(format!("Zero-length line in shape {}", shape.id));
            }
        }
        ShapeKind::Polygon {
            center,
            radius,
            sides,
        } => {
            if !center.is_finite() || !radius.is_finite() {
                report.errors.push(ValidationError::NonFiniteCoordinate(
                    shape.id,
                    "Polygon parameters non-finite".to_string(),
                ));
            } else if *sides < 3 {
                report.errors.push(ValidationError::InvalidGeometry(
                    shape.id,
                    format!("Polygon must have at least 3 sides, got {}", sides),
                ));
            }
        }
        ShapeKind::Star {
            center,
            outer_radius,
            inner_radius,
            points,
        } => {
            if !center.is_finite() || !outer_radius.is_finite() || !inner_radius.is_finite() {
                report.errors.push(ValidationError::NonFiniteCoordinate(
                    shape.id,
                    "Star parameters non-finite".to_string(),
                ));
            } else if *points < 3 {
                report.errors.push(ValidationError::InvalidGeometry(
                    shape.id,
                    format!("Star must have at least 3 points, got {}", points),
                ));
            }
        }
        ShapeKind::Spiral {
            center,
            turns,
            expansion,
        } => {
            if !center.is_finite() || !turns.is_finite() || !expansion.is_finite() {
                report.errors.push(ValidationError::NonFiniteCoordinate(
                    shape.id,
                    "Spiral parameters non-finite".to_string(),
                ));
            }
        }
        ShapeKind::Text {
            font_size,
            position,
            ..
        } => {
            if !position.is_finite() || !font_size.is_finite() || *font_size <= 0.0 {
                report.errors.push(ValidationError::NonFiniteCoordinate(
                    shape.id,
                    "Text position or font_size invalid".to_string(),
                ));
            }
        }
        ShapeKind::Group { shapes } => {
            if shapes.is_empty() {
                report.errors.push(ValidationError::EmptyGroup(shape.id));
            } else {
                for child in shapes {
                    validate_shape_recursive(child, seen_ids, report, depth + 1);
                }
            }
        }
    }
}

fn validate_rect(id: ShapeId, rect: &Rect, corner_radius: f64, report: &mut ValidationReport) {
    if !rect.min().is_finite() || !rect.max().is_finite() || !corner_radius.is_finite() {
        report.errors.push(ValidationError::NonFiniteCoordinate(
            id,
            "Rectangle bounds or corner_radius contain NaN/Infinity".to_string(),
        ));
    } else if rect.width() < 0.0 || rect.height() < 0.0 {
        report.errors.push(ValidationError::InvalidGeometry(
            id,
            "Rectangle has negative width or height".to_string(),
        ));
    } else if rect.min().x.abs() > MAX_VALID_COORDINATE_MM
        || rect.max().x.abs() > MAX_VALID_COORDINATE_MM
        || rect.min().y.abs() > MAX_VALID_COORDINATE_MM
        || rect.max().y.abs() > MAX_VALID_COORDINATE_MM
    {
        report.errors.push(ValidationError::CoordinateOutOfBounds(
            id,
            rect.min().x,
            MAX_VALID_COORDINATE_MM,
        ));
    }
}

fn validate_polycurve(id: ShapeId, poly: &core_geom::Polycurve, report: &mut ValidationReport) {
    for (s_idx, sub) in poly.subpaths.iter().enumerate() {
        for (n_idx, node) in sub.nodes.iter().enumerate() {
            if !node.point.is_finite() {
                report.errors.push(ValidationError::NonFiniteCoordinate(
                    id,
                    format!("Subpath {} Node {} anchor is non-finite", s_idx, n_idx),
                ));
            }
            if node.point.x.abs() > MAX_VALID_COORDINATE_MM {
                report.errors.push(ValidationError::CoordinateOutOfBounds(
                    id,
                    node.point.x,
                    MAX_VALID_COORDINATE_MM,
                ));
            }
            if node.point.y.abs() > MAX_VALID_COORDINATE_MM {
                report.errors.push(ValidationError::CoordinateOutOfBounds(
                    id,
                    node.point.y,
                    MAX_VALID_COORDINATE_MM,
                ));
            }
            if let Some(h) = node.handle_in {
                if !h.is_finite() {
                    report.errors.push(ValidationError::NonFiniteCoordinate(
                        id,
                        format!("Subpath {} Node {} handle_in is non-finite", s_idx, n_idx),
                    ));
                }
            }
            if let Some(h) = node.handle_out {
                if !h.is_finite() {
                    report.errors.push(ValidationError::NonFiniteCoordinate(
                        id,
                        format!("Subpath {} Node {} handle_out is non-finite", s_idx, n_idx),
                    ));
                }
            }
        }
    }
}
