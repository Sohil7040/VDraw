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
    /// Raw world position in millimeters.
    pub world_pos: Point2,
    /// Snapped world position in millimeters (equals world_pos if no snap occurred).
    pub snapped_pos: Point2,
    /// Screen position in pixels.
    pub screen_pos: egui::Pos2,
    /// Keyboard modifier flags.
    pub modifiers: ModifiersState,
    /// Whether this is a primary (left) button action.
    pub primary: bool,
    /// Viewport zoom scale.
    pub zoom: f64,
    /// Document page dimensions in mm (width, height).
    pub page_size: Option<(f64, f64)>,
}

impl PointerEvent {
    pub fn new(
        world_pos: Point2,
        screen_pos: egui::Pos2,
        modifiers: ModifiersState,
        primary: bool,
    ) -> Self {
        Self {
            world_pos,
            snapped_pos: world_pos,
            screen_pos,
            modifiers,
            primary,
            zoom: 1.0,
            page_size: None,
        }
    }

    pub fn with_snapped_pos(mut self, snapped: Point2) -> Self {
        self.snapped_pos = snapped;
        self
    }

    pub fn with_viewport(mut self, zoom: f64, page_size: Option<(f64, f64)>) -> Self {
        self.zoom = zoom;
        self.page_size = page_size;
        self
    }

    /// Return the active position (snapped position, or raw if Alt is pressed to suppress snap).
    #[inline]
    pub fn pos(&self) -> Point2 {
        if self.modifiers.alt {
            self.world_pos
        } else {
            self.snapped_pos
        }
    }
}
