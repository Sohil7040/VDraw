use core_geom::Point2;

/// Key modifier state for pointer and keyboard events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ModifiersState {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

/// Pointer event passed to the active tool.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointerEvent {
    /// World position in millimeters.
    pub world_pos: Point2,
    /// Screen position in pixels.
    pub screen_pos: egui::Pos2,
    /// Keyboard modifier flags.
    pub modifiers: ModifiersState,
    /// Whether this is a primary (left) button action.
    pub primary: bool,
}
