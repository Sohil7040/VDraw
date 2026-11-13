use core_doc::{Color, Document, Shape, ShapeId};
use core_geom::{Point2, Rect};
use io::*;

#[test]
fn test_vdraw_roundtrip_with_validation() {
    let mut doc = Document::new("My Drawing");
    let page = doc.active_page_mut().unwrap();
    page.width_mm = 210.0;
    page.height_mm = 297.0;

    let rect = Shape::rectangle(
        ShapeId(1),
        Rect::from_origin_size(Point2::new(10.0, 10.0), 100.0, 50.0),
        5.0,
    );
    page.layers[0].shapes.push(rect);

    let temp_file = std::env::temp_dir().join("test_doc.vdraw");
    save_vdraw(&doc, &temp_file).unwrap();

    let loaded = load_vdraw(&temp_file).unwrap();
    assert_eq!(loaded.title, "My Drawing");
    let loaded_page = loaded.active_page().unwrap();
    assert_eq!(loaded_page.layers[0].shapes.len(), 1);

    let _ = std::fs::remove_file(temp_file);
}

#[test]
fn test_svg_export_and_import_roundtrip() {
    let mut doc = Document::new("SVG Test");
    let page = doc.active_page_mut().unwrap();

    let mut rect = Shape::rectangle(
        ShapeId(1),
        Rect::from_origin_size(Point2::new(20.0, 30.0), 80.0, 60.0),
        0.0,
    );
    rect.style.fill = Some(Color::RED);
    page.layers[0].shapes.push(rect);

    let svg = export_svg(&doc);
    assert!(svg.contains("<svg"));
    assert!(svg.contains("width=\"210.00mm\""));
    assert!(svg.contains("M 20.0000 30.0000"));
    assert!(svg.contains("fill=\"rgb(255,0,0)\""));

    let imported = import_svg(&svg).unwrap();
    let imp_page = imported.active_page().unwrap();
    assert_eq!(imp_page.layers[0].shapes.len(), 1);
    let s = &imp_page.layers[0].shapes[0];
    assert_eq!(s.style.fill, Some(Color::RED));
}

#[test]
fn test_dxf_export() {
    let mut doc = Document::new("DXF Test");
    let page = doc.active_page_mut().unwrap();

    let line = Shape::line(ShapeId(1), Point2::new(0.0, 0.0), Point2::new(100.0, 100.0));
    page.layers[0].shapes.push(line);

    let dxf = export_dxf(&doc);
    assert!(dxf.contains("SECTION\n  2\nHEADER"));
    assert!(dxf.contains("LINE"));
    assert!(dxf.contains("100.0000"));
    assert!(dxf.contains("EOF"));
}
