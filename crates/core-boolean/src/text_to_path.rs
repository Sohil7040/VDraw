//! Text to Outlines engine: parses TrueType and OpenType fonts using `ttf-parser`
//! and converts glyph contours directly into editable `Polycurve` vector paths.

use core_geom::{BezierNode, NodeType, Point2, Polycurve, Subpath};
use ttf_parser::{Face, OutlineBuilder};

use crate::error::TextError;

/// Embedded default sans-serif font for guaranteed out-of-the-box text conversion.
pub const DEFAULT_FONT_BYTES: &[u8] = include_bytes!("../../../assets/fonts/default.ttf");

/// Options for text layout and conversion.
#[derive(Debug, Clone)]
pub struct TextOptions {
    /// Font size (cap height / em height) in document millimeters.
    pub font_size_mm: f64,
    /// Additional letter spacing in millimeters (default 0.0).
    pub letter_spacing_mm: f64,
    /// Origin position (baseline start) in document millimeters.
    pub origin: Point2,
}

impl Default for TextOptions {
    fn default() -> Self {
        Self {
            font_size_mm: 20.0,
            letter_spacing_mm: 0.0,
            origin: Point2::new(0.0, 0.0),
        }
    }
}

/// Converts a text string into an editable `Polycurve` using font bytes.
pub fn text_to_polycurve(
    text: &str,
    font_bytes: &[u8],
    options: &TextOptions,
) -> Result<Polycurve, TextError> {
    if text.is_empty() {
        return Err(TextError::EmptyText);
    }

    let face =
        Face::parse(font_bytes, 0).map_err(|e| TextError::FontParseError(format!("{:?}", e)))?;

    let units_per_em = face.units_per_em() as f64;
    let scale = options.font_size_mm / units_per_em;

    let mut all_subpaths = Vec::new();
    let mut cursor_x = options.origin.x;
    let cursor_y = options.origin.y;

    for c in text.chars() {
        if c == ' ' {
            let space_advance = (face.units_per_em() as f64 * 0.3) * scale;
            cursor_x += space_advance + options.letter_spacing_mm;
            continue;
        }

        let glyph_id = match face.glyph_index(c) {
            Some(id) => id,
            None => continue, // Skip unmapped glyphs gracefully
        };

        let mut builder = GlyphOutlineCollector {
            subpaths: Vec::new(),
            current_nodes: Vec::new(),
            start_point: Point2::new(0.0, 0.0),
            current_point: Point2::new(0.0, 0.0),
            offset_x: cursor_x,
            offset_y: cursor_y,
            scale,
        };

        if let Some(_bbox) = face.outline_glyph(glyph_id, &mut builder) {
            all_subpaths.extend(builder.finish());
        }

        // Advance cursor for next character
        let advance = face
            .glyph_hor_advance(glyph_id)
            .map(|a| a as f64 * scale)
            .unwrap_or(options.font_size_mm * 0.6);

        cursor_x += advance + options.letter_spacing_mm;
    }

    Ok(Polycurve {
        subpaths: all_subpaths,
    })
}

/// Converts a text string into a `Polycurve` using the bundled default font.
pub fn default_text_to_polycurve(
    text: &str,
    options: &TextOptions,
) -> Result<Polycurve, TextError> {
    text_to_polycurve(text, DEFAULT_FONT_BYTES, options)
}

struct GlyphOutlineCollector {
    subpaths: Vec<Subpath>,
    current_nodes: Vec<BezierNode>,
    start_point: Point2,
    current_point: Point2,
    offset_x: f64,
    offset_y: f64,
    scale: f64,
}

impl GlyphOutlineCollector {
    fn map_pt(&self, x: f32, y: f32) -> Point2 {
        Point2::new(
            self.offset_x + (x as f64) * self.scale,
            // Fonts have positive Y up; document has positive Y down
            self.offset_y - (y as f64) * self.scale,
        )
    }

    fn finish(mut self) -> Vec<Subpath> {
        self.flush_subpath(false);
        self.subpaths
    }

    fn flush_subpath(&mut self, is_closed: bool) {
        if self.current_nodes.len() >= 2 {
            self.subpaths.push(Subpath {
                nodes: std::mem::take(&mut self.current_nodes),
                closed: is_closed,
            });
        } else {
            self.current_nodes.clear();
        }
    }
}

impl OutlineBuilder for GlyphOutlineCollector {
    fn move_to(&mut self, x: f32, y: f32) {
        self.flush_subpath(false);
        let pt = self.map_pt(x, y);
        self.start_point = pt;
        self.current_point = pt;
        self.current_nodes.push(BezierNode::new(pt, NodeType::Cusp));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let pt = self.map_pt(x, y);
        self.current_point = pt;
        self.current_nodes.push(BezierNode::new(pt, NodeType::Cusp));
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let p0 = self.current_point;
        let q1 = self.map_pt(x1, y1);
        let p2 = self.map_pt(x, y);

        // Exact degree elevation from quadratic to cubic Bézier
        let c1 = Point2::new(
            p0.x + (2.0 / 3.0) * (q1.x - p0.x),
            p0.y + (2.0 / 3.0) * (q1.y - p0.y),
        );
        let c2 = Point2::new(
            p2.x + (2.0 / 3.0) * (q1.x - p2.x),
            p2.y + (2.0 / 3.0) * (q1.y - p2.y),
        );

        if let Some(prev) = self.current_nodes.last_mut() {
            prev.handle_out = Some(c1);
        }

        let mut next_node = BezierNode::new(p2, NodeType::Smooth);
        next_node.handle_in = Some(c2);
        self.current_nodes.push(next_node);

        self.current_point = p2;
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let c1 = self.map_pt(x1, y1);
        let c2 = self.map_pt(x2, y2);
        let p3 = self.map_pt(x, y);

        if let Some(prev) = self.current_nodes.last_mut() {
            prev.handle_out = Some(c1);
        }

        let mut next_node = BezierNode::new(p3, NodeType::Smooth);
        next_node.handle_in = Some(c2);
        self.current_nodes.push(next_node);

        self.current_point = p3;
    }

    fn close(&mut self) {
        self.flush_subpath(true);
    }
}
