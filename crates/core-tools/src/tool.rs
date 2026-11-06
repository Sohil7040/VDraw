use crate::event::PointerEvent;
use core_doc::{Document, UndoStack};
use core_geom::Point2;

/// Unique identifier for each tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolId {
    Pick,
    FreeTransform,
    Pen,
    Line,
    Freehand,
    Rectangle,
    Ellipse,
    Polygon,
    Spiral,
    Eraser,
    Knife,
    Text,
    Zoom,
    Pan,
}

impl ToolId {
    pub fn label(self) -> &'static str {
        match self {
            ToolId::Pick => "Pick",
            ToolId::FreeTransform => "Free Transform",
            ToolId::Pen => "Pen",
            ToolId::Line => "2-Point Line",
            ToolId::Freehand => "Freehand",
            ToolId::Rectangle => "Rectangle",
            ToolId::Ellipse => "Ellipse",
            ToolId::Polygon => "Polygon",
            ToolId::Spiral => "Spiral",
            ToolId::Eraser => "Eraser",
            ToolId::Knife => "Knife",
            ToolId::Text => "Text",
            ToolId::Zoom => "Zoom",
            ToolId::Pan => "Pan",
        }
    }

    pub fn shortcut(self) -> &'static str {
        match self {
            ToolId::Pick => "V",
            ToolId::FreeTransform => "Tab",
            ToolId::Pen => "B",
            ToolId::Line => "L",
            ToolId::Freehand => "F5",
            ToolId::Rectangle => "F6",
            ToolId::Ellipse => "F7",
            ToolId::Polygon => "Y",
            ToolId::Spiral => "A",
            ToolId::Eraser => "X",
            ToolId::Knife => "K",
            ToolId::Text => "T",
            ToolId::Zoom => "Z",
            ToolId::Pan => "H",
        }
    }
}

/// The Tool trait governing tool interactions, overlays, and state changes.
pub trait Tool: std::fmt::Debug + Send + Sync {
    fn id(&self) -> ToolId;

    fn on_pointer_down(
        &mut self,
        doc: &mut Document,
        undo: &mut UndoStack,
        event: &PointerEvent,
    ) -> bool;

    fn on_pointer_move(
        &mut self,
        doc: &mut Document,
        undo: &mut UndoStack,
        event: &PointerEvent,
    ) -> bool;

    fn on_pointer_up(
        &mut self,
        doc: &mut Document,
        undo: &mut UndoStack,
        event: &PointerEvent,
    ) -> bool;

    fn on_cancel(&mut self, _doc: &mut Document) {}

    fn cursor(&self) -> egui::CursorIcon {
        egui::CursorIcon::Default
    }

    fn draw_overlay(
        &self,
        _painter: &egui::Painter,
        _doc: &Document,
        _to_screen: &dyn Fn(Point2) -> egui::Pos2,
    ) {
    }
}
