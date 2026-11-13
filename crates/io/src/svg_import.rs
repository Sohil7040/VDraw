//! Pure Rust SVG Importer.
//!
//! Parses standard SVG geometry (rect, circle, ellipse, line, polyline, polygon, path)
//! into VDraw document shapes, preserving bezier curves, colors, and strokes.

use crate::error::IoError;
use core_doc::color::Color;
use core_doc::style::{Stroke, Style};
use core_doc::{Document, Layer, Shape, ShapeId};
use core_geom::{BezierNode, NodeType, Point2, Polycurve, Rect, Subpath};
use std::path::Path;

/// Import document from an SVG string.
pub fn import_svg(svg_content: &str) -> Result<Document, IoError> {
    let mut doc = Document::new("Imported SVG");
    let mut shapes = Vec::new();
    let mut next_id = 1u64;

    // 1. Extract viewBox / page dimensions if available
    let (width, height) = parse_svg_dimensions(svg_content).unwrap_or((210.0, 297.0));
    if let Some(page) = doc.active_page_mut() {
        page.width_mm = width;
        page.height_mm = height;
    }

    // 2. Tokenize tags by searching for '<' and '>'
    let mut cursor = 0;
    while let Some(start) = svg_content[cursor..].find('<') {
        let tag_start = cursor + start;
        let Some(end) = svg_content[tag_start..].find('>') else {
            break;
        };
        let tag_end = tag_start + end;
        cursor = tag_end + 1;

        let tag_str = &svg_content[tag_start + 1..tag_end].trim();
        if tag_str.starts_with('/') || tag_str.starts_with('?') || tag_str.starts_with('!') {
            continue;
        }

        let tag_name = tag_str
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim_end_matches('/');

        match tag_name {
            "rect" => {
                if let Some(shape) = parse_rect(tag_str, ShapeId(next_id)) {
                    shapes.push(shape);
                    next_id += 1;
                }
            }
            "circle" => {
                if let Some(shape) = parse_circle(tag_str, ShapeId(next_id)) {
                    shapes.push(shape);
                    next_id += 1;
                }
            }
            "ellipse" => {
                if let Some(shape) = parse_ellipse(tag_str, ShapeId(next_id)) {
                    shapes.push(shape);
                    next_id += 1;
                }
            }
            "line" => {
                if let Some(shape) = parse_line(tag_str, ShapeId(next_id)) {
                    shapes.push(shape);
                    next_id += 1;
                }
            }
            "polyline" | "polygon" => {
                let closed = tag_name == "polygon";
                if let Some(shape) = parse_poly_points(tag_str, closed, ShapeId(next_id)) {
                    shapes.push(shape);
                    next_id += 1;
                }
            }
            "path" => {
                if let Some(shape) = parse_path(tag_str, ShapeId(next_id)) {
                    shapes.push(shape);
                    next_id += 1;
                }
            }
            _ => {}
        }
    }

    if let Some(page) = doc.active_page_mut() {
        if page.layers.is_empty() {
            page.layers.push(Layer {
                name: "Layer 1".to_string(),
                visible: true,
                locked: false,
                shapes,
            });
        } else {
            page.layers[0].shapes = shapes;
        }
    }

    Ok(doc)
}

/// Load document from an SVG file.
pub fn load_svg(path: &Path) -> Result<Document, IoError> {
    let content = std::fs::read_to_string(path)?;
    import_svg(&content)
}

fn parse_svg_dimensions(svg: &str) -> Option<(f64, f64)> {
    if let Some(vb) = get_attr(svg, "viewBox") {
        let parts: Vec<f64> = vb
            .split(|c: char| c.is_whitespace() || c == ',')
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect();
        if parts.len() >= 4 && parts[2] > 0.0 && parts[3] > 0.0 {
            return Some((parts[2], parts[3]));
        }
    }
    let w = parse_length(get_attr(svg, "width")?.as_str())?;
    let h = parse_length(get_attr(svg, "height")?.as_str())?;
    Some((w, h))
}

fn parse_length(s: &str) -> Option<f64> {
    let s = s.trim();
    if let Some(stripped) = s.strip_suffix("mm") {
        stripped.parse().ok()
    } else if let Some(stripped) = s.strip_suffix("in") {
        stripped.parse::<f64>().ok().map(|v| v * 25.4)
    } else if let Some(stripped) = s.strip_suffix("px") {
        stripped.parse::<f64>().ok().map(|v| v * 25.4 / 96.0)
    } else {
        s.parse().ok()
    }
}

fn get_attr(tag: &str, attr: &str) -> Option<String> {
    for (i, _) in tag.match_indices(attr) {
        let is_start = i == 0 || tag.as_bytes()[i - 1].is_ascii_whitespace();
        if !is_start {
            continue;
        }
        let rest = tag[i + attr.len()..].trim_start();
        if let Some(rest_after_eq) = rest.strip_prefix('=') {
            let rest_trimmed = rest_after_eq.trim_start();
            if let Some(quote) = rest_trimmed.chars().next() {
                if quote == '"' || quote == '\'' {
                    let val_content = &rest_trimmed[1..];
                    if let Some(end) = val_content.find(quote) {
                        return Some(val_content[..end].to_string());
                    }
                }
            }
        }
    }
    None
}

fn parse_style(tag: &str) -> Style {
    let mut style = Style::default();

    // Check fill
    if let Some(fill) = get_attr(tag, "fill") {
        if fill == "none" {
            style.fill = None;
        } else if let Some(c) = parse_color(&fill) {
            style.fill = Some(c);
        }
    }

    // Check stroke
    if let Some(stroke) = get_attr(tag, "stroke") {
        if stroke != "none" {
            let color = parse_color(&stroke).unwrap_or(Color::BLACK);
            let width = get_attr(tag, "stroke-width")
                .and_then(|w| parse_length(&w))
                .unwrap_or(0.2);
            style.stroke = Some(Stroke {
                color,
                width,
                ..Default::default()
            });
        } else {
            style.stroke = None;
        }
    }

    style
}

fn parse_color(s: &str) -> Option<Color> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix('#') {
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            return Some(Color { r, g, b, a: 255 });
        } else if hex.len() == 3 {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
            return Some(Color { r, g, b, a: 255 });
        }
    } else if let Some(inner) = s.strip_prefix("rgb(").and_then(|s| s.strip_suffix(')')) {
        let nums: Vec<u8> = inner
            .split(',')
            .filter_map(|v| v.trim().parse().ok())
            .collect();
        if nums.len() >= 3 {
            return Some(Color {
                r: nums[0],
                g: nums[1],
                b: nums[2],
                a: 255,
            });
        }
    } else {
        match s.to_lowercase().as_str() {
            "black" => return Some(Color::BLACK),
            "white" => return Some(Color::WHITE),
            "red" => return Some(Color::RED),
            "green" => return Some(Color::GREEN),
            "blue" => return Some(Color::BLUE),
            _ => {}
        }
    }
    None
}

fn parse_rect(tag: &str, id: ShapeId) -> Option<Shape> {
    let x = get_attr(tag, "x")
        .and_then(|v| parse_length(&v))
        .unwrap_or(0.0);
    let y = get_attr(tag, "y")
        .and_then(|v| parse_length(&v))
        .unwrap_or(0.0);
    let width = get_attr(tag, "width").and_then(|v| parse_length(&v))?;
    let height = get_attr(tag, "height").and_then(|v| parse_length(&v))?;
    let rx = get_attr(tag, "rx")
        .and_then(|v| parse_length(&v))
        .unwrap_or(0.0);

    let mut shape = Shape::rectangle(
        id,
        Rect::from_origin_size(Point2::new(x, y), width, height),
        rx,
    );
    shape.style = parse_style(tag);
    Some(shape)
}

fn parse_circle(tag: &str, id: ShapeId) -> Option<Shape> {
    let cx = get_attr(tag, "cx")
        .and_then(|v| parse_length(&v))
        .unwrap_or(0.0);
    let cy = get_attr(tag, "cy")
        .and_then(|v| parse_length(&v))
        .unwrap_or(0.0);
    let r = get_attr(tag, "r").and_then(|v| parse_length(&v))?;

    let mut shape = Shape::ellipse(id, Point2::new(cx, cy), r, r);
    shape.style = parse_style(tag);
    Some(shape)
}

fn parse_ellipse(tag: &str, id: ShapeId) -> Option<Shape> {
    let cx = get_attr(tag, "cx")
        .and_then(|v| parse_length(&v))
        .unwrap_or(0.0);
    let cy = get_attr(tag, "cy")
        .and_then(|v| parse_length(&v))
        .unwrap_or(0.0);
    let rx = get_attr(tag, "rx").and_then(|v| parse_length(&v))?;
    let ry = get_attr(tag, "ry").and_then(|v| parse_length(&v))?;

    let mut shape = Shape::ellipse(id, Point2::new(cx, cy), rx, ry);
    shape.style = parse_style(tag);
    Some(shape)
}

fn parse_line(tag: &str, id: ShapeId) -> Option<Shape> {
    let x1 = get_attr(tag, "x1")
        .and_then(|v| parse_length(&v))
        .unwrap_or(0.0);
    let y1 = get_attr(tag, "y1")
        .and_then(|v| parse_length(&v))
        .unwrap_or(0.0);
    let x2 = get_attr(tag, "x2")
        .and_then(|v| parse_length(&v))
        .unwrap_or(0.0);
    let y2 = get_attr(tag, "y2")
        .and_then(|v| parse_length(&v))
        .unwrap_or(0.0);

    let mut shape = Shape::line(id, Point2::new(x1, y1), Point2::new(x2, y2));
    shape.style = parse_style(tag);
    Some(shape)
}

fn parse_poly_points(tag: &str, closed: bool, id: ShapeId) -> Option<Shape> {
    let pts_str = get_attr(tag, "points")?;
    let nums: Vec<f64> = pts_str
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();

    if nums.len() < 4 {
        return None;
    }

    let mut subpath = Subpath::new(closed);
    for chunk in nums.chunks_exact(2) {
        subpath.push_node(BezierNode::new(
            Point2::new(chunk[0], chunk[1]),
            NodeType::Cusp,
        ));
    }

    let poly = Polycurve {
        subpaths: vec![subpath],
    };
    let mut shape = Shape::path(id, poly);
    shape.style = parse_style(tag);
    Some(shape)
}

/// Tokenize and parse standard SVG path data (M, L, H, V, C, S, Z).
fn parse_path(tag: &str, id: ShapeId) -> Option<Shape> {
    let d = get_attr(tag, "d")?;
    let poly = parse_svg_path_data(&d)?;
    let mut shape = Shape::path(id, poly);
    shape.style = parse_style(tag);
    Some(shape)
}

/// Parse SVG path data string `d` into `Polycurve`.
pub fn parse_svg_path_data(d: &str) -> Option<Polycurve> {
    let mut subpaths = Vec::new();
    let mut current_subpath = Subpath::new(false);
    let mut cur = Point2::ZERO;
    let mut start_pt = Point2::ZERO;

    let chars: Vec<char> = d.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        while i < chars.len() && (chars[i].is_whitespace() || chars[i] == ',') {
            i += 1;
        }
        if i >= chars.len() {
            break;
        }

        let cmd = chars[i];
        i += 1;

        match cmd {
            'M' | 'm' => {
                let coords = read_numbers(&chars, &mut i, 2);
                if coords.len() == 2 {
                    let pt = if cmd == 'm' {
                        Point2::new(cur.x + coords[0], cur.y + coords[1])
                    } else {
                        Point2::new(coords[0], coords[1])
                    };
                    if !current_subpath.is_empty() {
                        subpaths.push(current_subpath);
                        current_subpath = Subpath::new(false);
                    }
                    current_subpath.push_node(BezierNode::new(pt, NodeType::Cusp));
                    cur = pt;
                    start_pt = pt;
                }
            }
            'L' | 'l' => {
                let coords = read_numbers(&chars, &mut i, 2);
                if coords.len() == 2 {
                    let pt = if cmd == 'l' {
                        Point2::new(cur.x + coords[0], cur.y + coords[1])
                    } else {
                        Point2::new(coords[0], coords[1])
                    };
                    current_subpath.push_node(BezierNode::new(pt, NodeType::Cusp));
                    cur = pt;
                }
            }
            'H' | 'h' => {
                let coords = read_numbers(&chars, &mut i, 1);
                if !coords.is_empty() {
                    let x = if cmd == 'h' {
                        cur.x + coords[0]
                    } else {
                        coords[0]
                    };
                    let pt = Point2::new(x, cur.y);
                    current_subpath.push_node(BezierNode::new(pt, NodeType::Cusp));
                    cur = pt;
                }
            }
            'V' | 'v' => {
                let coords = read_numbers(&chars, &mut i, 1);
                if !coords.is_empty() {
                    let y = if cmd == 'v' {
                        cur.y + coords[0]
                    } else {
                        coords[0]
                    };
                    let pt = Point2::new(cur.x, y);
                    current_subpath.push_node(BezierNode::new(pt, NodeType::Cusp));
                    cur = pt;
                }
            }
            'C' | 'c' => {
                let coords = read_numbers(&chars, &mut i, 6);
                if coords.len() == 6 {
                    let (p1, p2, p3) = if cmd == 'c' {
                        (
                            Point2::new(cur.x + coords[0], cur.y + coords[1]),
                            Point2::new(cur.x + coords[2], cur.y + coords[3]),
                            Point2::new(cur.x + coords[4], cur.y + coords[5]),
                        )
                    } else {
                        (
                            Point2::new(coords[0], coords[1]),
                            Point2::new(coords[2], coords[3]),
                            Point2::new(coords[4], coords[5]),
                        )
                    };
                    // Attach outgoing handle to previous node
                    if let Some(last) = current_subpath.nodes.last_mut() {
                        last.handle_out = Some(p1);
                    }
                    // Create new node with incoming handle
                    let node = BezierNode::with_handles(p3, Some(p2), None, NodeType::Smooth);
                    current_subpath.push_node(node);
                    cur = p3;
                }
            }
            'Z' | 'z' => {
                current_subpath.closed = true;
                cur = start_pt;
                subpaths.push(current_subpath);
                current_subpath = Subpath::new(false);
            }
            _ => {}
        }
    }

    if !current_subpath.is_empty() {
        subpaths.push(current_subpath);
    }

    if subpaths.is_empty() {
        None
    } else {
        Some(Polycurve { subpaths })
    }
}

fn read_numbers(chars: &[char], i: &mut usize, max_count: usize) -> Vec<f64> {
    let mut numbers = Vec::with_capacity(max_count);
    while numbers.len() < max_count && *i < chars.len() {
        while *i < chars.len() && (chars[*i].is_whitespace() || chars[*i] == ',') {
            *i += 1;
        }
        if *i >= chars.len() || chars[*i].is_alphabetic() {
            break;
        }

        let start = *i;
        if chars[*i] == '-' || chars[*i] == '+' {
            *i += 1;
        }
        while *i < chars.len() && (chars[*i].is_ascii_digit() || chars[*i] == '.') {
            *i += 1;
        }
        let s: String = chars[start..*i].iter().collect();
        if let Ok(n) = s.parse::<f64>() {
            numbers.push(n);
        } else {
            break;
        }
    }
    numbers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_svg_rect_and_circle() {
        let svg = r##"
            <svg width="200mm" height="100mm" viewBox="0 0 200 100">
                <rect x="10" y="20" width="50" height="30" rx="2" fill="#ff0000" stroke="#000000" stroke-width="1.5" />
                <circle cx="100" cy="50" r="25" fill="none" stroke="#00ff00" />
            </svg>
        "##;

        let doc = import_svg(svg).unwrap();
        let page = doc.active_page().unwrap();
        assert_eq!(page.width_mm, 200.0);
        assert_eq!(page.height_mm, 100.0);
        assert_eq!(page.layers[0].shapes.len(), 2);

        let s1 = &page.layers[0].shapes[0];
        assert_eq!(
            s1.style.fill,
            Some(Color {
                r: 255,
                g: 0,
                b: 0,
                a: 255
            })
        );
        assert_eq!(s1.style.stroke.as_ref().unwrap().width, 1.5);

        let s2 = &page.layers[0].shapes[1];
        assert_eq!(s2.style.fill, None);
        assert_eq!(
            s2.style.stroke.as_ref().unwrap().color,
            Color {
                r: 0,
                g: 255,
                b: 0,
                a: 255
            }
        );
    }

    #[test]
    fn test_parse_svg_path_data() {
        let d = "M 10 10 L 50 10 C 60 10 70 20 70 30 L 70 70 Z";
        let poly = parse_svg_path_data(d).unwrap();
        assert_eq!(poly.subpaths.len(), 1);
        assert!(poly.subpaths[0].closed);
        assert_eq!(poly.subpaths[0].nodes.len(), 4);
    }
}
