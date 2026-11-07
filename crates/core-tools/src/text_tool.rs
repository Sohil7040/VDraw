//! Text tool: click on canvas to place vector text contours.

use crate::event::PointerEvent;
use crate::tool::{Tool, ToolId};
use core_boolean::apply_text_to_document;
use core_doc::{Document, UndoStack};
use core_geom::Point2;

#[derive(Debug)]
pub struct TextTool {
    pub pending_text: String,
    pub font_size_mm: f64,
    pub click_pos: Option<Point2>,
    pub show_dialog: bool,
}

impl Default for TextTool {
    fn default() -> Self {
        Self::new()
    }
}

impl TextTool {
    pub fn new() -> Self {
        Self {
            pending_text: "VDraw Text".to_string(),
            font_size_mm: 20.0,
            click_pos: None,
            show_dialog: false,
        }
    }
}

impl Tool for TextTool {
    fn id(&self) -> ToolId {
        ToolId::Text
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn on_pointer_down(
        &mut self,
        doc: &mut Document,
        undo: &mut UndoStack,
        event: &PointerEvent,
    ) -> bool {
        let pos = event.pos();
        self.click_pos = Some(pos);

        // Instantly generate sample text or open text dialog
        let _ = apply_text_to_document(doc, undo, &self.pending_text, self.font_size_mm, pos);
        true
    }

    fn on_pointer_move(
        &mut self,
        _doc: &mut Document,
        _undo: &mut UndoStack,
        event: &PointerEvent,
    ) -> bool {
        self.click_pos = Some(event.pos());
        false
    }

    fn on_pointer_up(
        &mut self,
        _doc: &mut Document,
        _undo: &mut UndoStack,
        _event: &PointerEvent,
    ) -> bool {
        false
    }

    fn cursor(&self) -> egui::CursorIcon {
        egui::CursorIcon::Text
    }

    fn on_cancel(&mut self, _doc: &mut Document) {
        self.click_pos = None;
    }
}
