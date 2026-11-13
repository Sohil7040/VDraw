use crate::error::IoError;
use core_doc::Document;
use std::path::Path;

/// Export document as an industry-standard SVG file.
///
/// Compatible with LaserWeb, LightBurn, Inkscape, and standard CAD/CAM laser workflows.
/// Applies shape transformations directly to coordinates so geometry is universally valid.
pub fn export_svg(doc: &Document) -> String {
    let mut out = String::with_capacity(16384);

    let (width_mm, height_mm) = if let Some(page) = doc.active_page() {
        (page.width_mm, page.height_mm)
    } else {
        (210.0, 297.0) // Fallback A4
    };

    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?>\n");
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:inkscape=\"http://www.inkscape.org/namespaces/inkscape\" width=\"{:.2}mm\" height=\"{:.2}mm\" viewBox=\"0 0 {:.2} {:.2}\" version=\"1.1\">\n",
        width_mm, height_mm, width_mm, height_mm
    ));

    let Some(page) = doc.active_page() else {
        out.push_str("</svg>\n");
        return out;
    };

    for (layer_idx, layer) in page.layers.iter().enumerate() {
        if !layer.visible {
            continue;
        }

        out.push_str(&format!(
            "  <g id=\"layer_{}\" inkscape:label=\"{}\" inkscape:groupmode=\"layer\">\n",
            layer_idx + 1,
            xml_escape(&layer.name)
        ));

        for shape in &layer.shapes {
            if !shape.visible {
                continue;
            }

            // Transform polycurve into world coordinate space
            let poly = shape.to_polycurve().transform(&shape.transform);
            let mut d = String::new();

            for subpath in &poly.subpaths {
                if subpath.nodes.is_empty() {
                    continue;
                }

                let p0 = subpath.nodes[0].point;
                d.push_str(&format!("M {:.4} {:.4} ", p0.x, p0.y));

                let cubics = subpath.to_cubics();
                for c in cubics {
                    // Check if segment is a straight line
                    let is_straight = c.p1.distance(c.p0) < 1e-4 && c.p2.distance(c.p3) < 1e-4;
                    if is_straight {
                        d.push_str(&format!("L {:.4} {:.4} ", c.p3.x, c.p3.y));
                    } else {
                        d.push_str(&format!(
                            "C {:.4} {:.4}, {:.4} {:.4}, {:.4} {:.4} ",
                            c.p1.x, c.p1.y, c.p2.x, c.p2.y, c.p3.x, c.p3.y
                        ));
                    }
                }

                if subpath.closed {
                    d.push_str("Z ");
                }
            }

            if d.is_empty() {
                continue;
            }

            // Format styles
            let (fill_str, fill_opacity) = if let Some(fill) = &shape.style.fill {
                (
                    format!("rgb({},{},{})", fill.r, fill.g, fill.b),
                    format!("{:.3}", fill.a as f64 / 255.0),
                )
            } else {
                ("none".to_string(), "1.0".to_string())
            };

            let (stroke_str, stroke_w, stroke_opacity) = if let Some(stroke) = &shape.style.stroke {
                (
                    format!(
                        "rgb({},{},{})",
                        stroke.color.r, stroke.color.g, stroke.color.b
                    ),
                    format!("{:.3}", stroke.width),
                    format!("{:.3}", stroke.color.a as f64 / 255.0),
                )
            } else {
                ("none".to_string(), "0".to_string(), "1.0".to_string())
            };

            out.push_str(&format!(
                "    <path id=\"shape_{}\" d=\"{}\" fill=\"{}\" fill-opacity=\"{}\" stroke=\"{}\" stroke-width=\"{}\" stroke-opacity=\"{}\" />\n",
                shape.id.0,
                d.trim(),
                fill_str,
                fill_opacity,
                stroke_str,
                stroke_w,
                stroke_opacity
            ));
        }

        out.push_str("  </g>\n");
    }

    out.push_str("</svg>\n");
    out
}

/// Save document as an SVG file at the specified path.
pub fn save_svg(doc: &Document, path: &Path) -> Result<(), IoError> {
    let svg = export_svg(doc);
    std::fs::write(path, svg)?;
    Ok(())
}

fn xml_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
