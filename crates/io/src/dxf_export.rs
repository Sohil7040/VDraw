//! AutoCAD DXF (R12 / R14 LWPOLYLINE) Exporter.
//!
//! Provides clean, standard DXF output compatible with AutoCAD, SolidWorks,
//! SheetCAM, LightBurn, Fusion 360, and industrial CNC machines.

use crate::error::IoError;
use core_doc::{Document, ShapeKind};
use std::path::Path;

/// Export document as standard AutoCAD DXF format.
pub fn export_dxf(doc: &Document) -> String {
    let mut out = String::with_capacity(16384);

    // 1. Header Section
    out.push_str("  0\nSECTION\n  2\nHEADER\n");
    out.push_str("  9\n$ACADVER\n  1\nAC1009\n"); // AutoCAD R12 standard compatibility
    out.push_str("  9\n$INSUNITS\n 70\n     4\n"); // 4 = Millimeters
    out.push_str("  0\nENDSEC\n");

    // 2. Tables Section (Layer definitions)
    out.push_str("  0\nSECTION\n  2\nTABLES\n");
    out.push_str("  0\nTABLE\n  2\nLAYER\n 70\n     1\n");

    let Some(page) = doc.active_page() else {
        out.push_str("  0\nENDTAB\n  0\nENDSEC\n  0\nSECTION\n  2\nBLOCKS\n  0\nENDSEC\n  0\nSECTION\n  2\nENTITIES\n  0\nENDSEC\n  0\nEOF\n");
        return out;
    };

    for layer in &page.layers {
        out.push_str("  0\nLAYER\n");
        out.push_str(&format!("  2\n{}\n", clean_dxf_str(&layer.name)));
        out.push_str(" 70\n     0\n"); // Standard flags
        out.push_str(" 62\n     7\n"); // White / default color
        out.push_str("  6\nCONTINUOUS\n");
    }
    out.push_str("  0\nENDTAB\n  0\nENDSEC\n");

    // 3. Blocks Section (Empty)
    out.push_str("  0\nSECTION\n  2\nBLOCKS\n  0\nENDSEC\n");

    // 4. Entities Section
    out.push_str("  0\nSECTION\n  2\nENTITIES\n");

    for layer in &page.layers {
        if !layer.visible {
            continue;
        }
        let layer_name = clean_dxf_str(&layer.name);

        for shape in &layer.shapes {
            if !shape.visible {
                continue;
            }

            match &shape.kind {
                ShapeKind::Line { start, end } if shape.transform.is_identity() => {
                    out.push_str("  0\nLINE\n");
                    out.push_str(&format!("  8\n{}\n", layer_name));
                    out.push_str(&format!(
                        " 10\n{:.4}\n 20\n{:.4}\n 30\n0.0\n",
                        start.x, start.y
                    ));
                    out.push_str(&format!(" 11\n{:.4}\n 21\n{:.4}\n 31\n0.0\n", end.x, end.y));
                }
                ShapeKind::Ellipse { center, rx, ry }
                    if (rx - ry).abs() < 1e-4 && shape.transform.is_identity() =>
                {
                    // Exact CIRCLE entity
                    out.push_str("  0\nCIRCLE\n");
                    out.push_str(&format!("  8\n{}\n", layer_name));
                    out.push_str(&format!(
                        " 10\n{:.4}\n 20\n{:.4}\n 30\n0.0\n",
                        center.x, center.y
                    ));
                    out.push_str(&format!(" 40\n{:.4}\n", rx));
                }
                _ => {
                    // Export as Polyline / LWPOLYLINE
                    let poly = shape.to_polycurve().transform(&shape.transform);
                    for subpath in &poly.subpaths {
                        let pts = subpath.flatten(0.02);
                        if pts.len() < 2 {
                            continue;
                        }

                        out.push_str("  0\nLWPOLYLINE\n");
                        out.push_str(&format!("  8\n{}\n", layer_name));
                        out.push_str(&format!(" 90\n{}\n", pts.len()));
                        out.push_str(&format!(" 70\n{}\n", if subpath.closed { 1 } else { 0 }));

                        for p in &pts {
                            out.push_str(&format!(" 10\n{:.4}\n 20\n{:.4}\n", p.x, p.y));
                        }
                    }
                }
            }
        }
    }

    out.push_str("  0\nENDSEC\n  0\nEOF\n");
    out
}

/// Save document as a DXF file at the specified path.
pub fn save_dxf(doc: &Document, path: &Path) -> Result<(), IoError> {
    let dxf = export_dxf(doc);
    std::fs::write(path, dxf)?;
    Ok(())
}

fn clean_dxf_str(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_doc::Shape;
    use core_geom::{Point2, Rect};

    #[test]
    fn test_export_dxf_lines_and_polyline() {
        let mut doc = Document::default();
        let page = doc.active_page_mut().unwrap();

        let l = Shape::line(
            core_doc::ShapeId(1),
            Point2::new(0.0, 0.0),
            Point2::new(50.0, 50.0),
        );
        let r = Shape::rectangle(
            core_doc::ShapeId(2),
            Rect::from_origin_size(Point2::new(10.0, 10.0), 40.0, 30.0),
            0.0,
        );

        page.layers[0].shapes.push(l);
        page.layers[0].shapes.push(r);

        let dxf = export_dxf(&doc);
        assert!(dxf.contains("SECTION\n  2\nHEADER"));
        assert!(dxf.contains("LINE"));
        assert!(dxf.contains("LWPOLYLINE"));
        assert!(dxf.contains("EOF"));
    }
}
