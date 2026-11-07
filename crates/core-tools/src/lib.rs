//! Tool state machine and interactive tool implementations for VDraw.

pub mod ellipse;
pub mod event;
pub mod freehand;
pub mod line;
pub mod pen;
pub mod pick;
pub mod polygon;
pub mod rectangle;
pub mod shape_tool;
pub mod spiral;
pub mod star;
pub mod tool;

pub use ellipse::EllipseTool;
pub use event::{ModifiersState, PointerEvent};
pub use freehand::FreehandTool;
pub use line::LineTool;
pub use pen::PenTool;
pub use pick::PickTool;
pub use polygon::PolygonTool;
pub use rectangle::RectangleTool;
pub use shape_tool::{SelectedNode, ShapeTool};
pub use spiral::SpiralTool;
pub use star::StarTool;
pub use tool::{Tool, ToolId};

use core_doc::{Document, UndoStack};
use core_geom::Point2;

/// Manages the currently active tool instance and state transitions.
#[derive(Debug)]
pub struct ToolManager {
    active_tool: Box<dyn Tool>,
    active_id: ToolId,
}

impl Default for ToolManager {
    fn default() -> Self {
        Self::new(ToolId::Pick)
    }
}

impl ToolManager {
    pub fn new(initial: ToolId) -> Self {
        let mut mgr = Self {
            active_tool: Box::new(PickTool::new()),
            active_id: ToolId::Pick,
        };
        mgr.set_tool(initial);
        mgr
    }

    pub fn active_id(&self) -> ToolId {
        self.active_id
    }

    pub fn set_tool(&mut self, id: ToolId) {
        if self.active_id == id {
            return;
        }
        self.active_id = id;
        self.active_tool = match id {
            ToolId::Pick => Box::new(PickTool::new()),
            ToolId::Shape => Box::new(ShapeTool::new()),
            ToolId::Rectangle => Box::new(RectangleTool::new()),
            ToolId::Ellipse => Box::new(EllipseTool::new()),
            ToolId::Line => Box::new(LineTool::new()),
            ToolId::Pen => Box::new(PenTool::new()),
            ToolId::Freehand => Box::new(FreehandTool::new()),
            ToolId::Polygon => Box::new(PolygonTool::new()),
            ToolId::Star => Box::new(StarTool::new()),
            ToolId::Spiral => Box::new(SpiralTool::new()),
            _ => Box::new(PickTool::new()),
        };
    }

    pub fn active_tool(&self) -> &dyn Tool {
        self.active_tool.as_ref()
    }

    pub fn active_tool_mut(&mut self) -> &mut dyn Tool {
        self.active_tool.as_mut()
    }

    pub fn shape_tool_mut(&mut self) -> Option<&mut ShapeTool> {
        self.active_tool.as_any_mut().downcast_mut::<ShapeTool>()
    }

    pub fn polygon_tool_mut(&mut self) -> Option<&mut PolygonTool> {
        self.active_tool.as_any_mut().downcast_mut::<PolygonTool>()
    }

    pub fn star_tool_mut(&mut self) -> Option<&mut StarTool> {
        self.active_tool.as_any_mut().downcast_mut::<StarTool>()
    }

    pub fn spiral_tool_mut(&mut self) -> Option<&mut SpiralTool> {
        self.active_tool.as_any_mut().downcast_mut::<SpiralTool>()
    }

    pub fn freehand_tool_mut(&mut self) -> Option<&mut FreehandTool> {
        self.active_tool.as_any_mut().downcast_mut::<FreehandTool>()
    }

    pub fn on_pointer_down(
        &mut self,
        doc: &mut Document,
        undo: &mut UndoStack,
        event: &PointerEvent,
    ) -> bool {
        self.active_tool.on_pointer_down(doc, undo, event)
    }

    pub fn on_pointer_move(
        &mut self,
        doc: &mut Document,
        undo: &mut UndoStack,
        event: &PointerEvent,
    ) -> bool {
        self.active_tool.on_pointer_move(doc, undo, event)
    }

    pub fn on_pointer_up(
        &mut self,
        doc: &mut Document,
        undo: &mut UndoStack,
        event: &PointerEvent,
    ) -> bool {
        self.active_tool.on_pointer_up(doc, undo, event)
    }

    pub fn on_cancel(&mut self, doc: &mut Document) {
        self.active_tool.on_cancel(doc);
    }

    pub fn cursor(&self) -> egui::CursorIcon {
        self.active_tool.cursor()
    }

    pub fn draw_overlay(
        &self,
        painter: &egui::Painter,
        doc: &Document,
        to_screen: &dyn Fn(Point2) -> egui::Pos2,
    ) {
        self.active_tool.draw_overlay(painter, doc, to_screen);
    }
}
