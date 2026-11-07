use crate::guideline::{Guideline, GuidelineId, GuidelineOrientation};
use crate::selection::Selection;
use crate::shape::{Shape, ShapeId};
use crate::units::Unit;
use core_geom::Rect;
use serde::{Deserialize, Serialize};

/// A layer holding a list of shapes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    pub shapes: Vec<Shape>,
}

impl Default for Layer {
    fn default() -> Self {
        Self {
            name: "Layer 1".to_string(),
            visible: true,
            locked: false,
            shapes: Vec::new(),
        }
    }
}

impl Layer {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    pub fn find_shape(&self, id: ShapeId) -> Option<&Shape> {
        self.shapes.iter().find(|s| s.id == id)
    }

    pub fn find_shape_mut(&mut self, id: ShapeId) -> Option<&mut Shape> {
        self.shapes.iter_mut().find(|s| s.id == id)
    }

    pub fn remove_shape(&mut self, id: ShapeId) -> Option<Shape> {
        if let Some(pos) = self.shapes.iter().position(|s| s.id == id) {
            Some(self.shapes.remove(pos))
        } else {
            None
        }
    }
}

/// A page within a document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Page {
    pub name: String,
    /// Page width in millimeters.
    pub width_mm: f64,
    /// Page height in millimeters.
    pub height_mm: f64,
    pub layers: Vec<Layer>,
    pub active_layer: usize,
}

impl Default for Page {
    fn default() -> Self {
        Self {
            name: "Page 1".to_string(),
            // A4 dimensions (210 x 297 mm)
            width_mm: 210.0,
            height_mm: 297.0,
            layers: vec![Layer::default()],
            active_layer: 0,
        }
    }
}

impl Page {
    pub fn new(name: impl Into<String>, width_mm: f64, height_mm: f64) -> Self {
        Self {
            name: name.into(),
            width_mm,
            height_mm,
            layers: vec![Layer::default()],
            active_layer: 0,
        }
    }

    pub fn active_layer(&self) -> Option<&Layer> {
        self.layers.get(self.active_layer)
    }

    pub fn active_layer_mut(&mut self) -> Option<&mut Layer> {
        self.layers.get_mut(self.active_layer)
    }

    pub fn find_shape(&self, id: ShapeId) -> Option<(&Shape, usize)> {
        for (idx, layer) in self.layers.iter().enumerate() {
            if let Some(shape) = layer.find_shape(id) {
                return Some((shape, idx));
            }
        }
        None
    }

    pub fn find_shape_mut(&mut self, id: ShapeId) -> Option<(&mut Shape, usize)> {
        for (idx, layer) in self.layers.iter_mut().enumerate() {
            if let Some(shape) = layer.find_shape_mut(id) {
                return Some((shape, idx));
            }
        }
        None
    }

    pub fn all_shapes(&self) -> impl Iterator<Item = &Shape> {
        self.layers.iter().flat_map(|l| l.shapes.iter())
    }
}

/// A vector document containing pages, shapes, layers, selection, and unit settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub title: String,
    pub unit: Unit,
    pub pages: Vec<Page>,
    pub active_page: usize,
    pub selection: Selection,
    pub next_shape_id: u64,
    pub guidelines: Vec<Guideline>,
    pub next_guideline_id: u64,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            title: "Untitled".to_string(),
            unit: Unit::Millimeter,
            pages: vec![Page::default()],
            active_page: 0,
            selection: Selection::default(),
            next_shape_id: 1,
            guidelines: Vec::new(),
            next_guideline_id: 1,
        }
    }
}

impl Document {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Default::default()
        }
    }

    pub fn allocate_id(&mut self) -> ShapeId {
        let id = ShapeId(self.next_shape_id);
        self.next_shape_id += 1;
        id
    }

    pub fn allocate_guideline_id(&mut self) -> GuidelineId {
        let id = GuidelineId(self.next_guideline_id);
        self.next_guideline_id += 1;
        id
    }

    pub fn add_guideline(&mut self, orientation: GuidelineOrientation) -> GuidelineId {
        let id = self.allocate_guideline_id();
        self.guidelines.push(Guideline {
            id,
            orientation,
            locked: false,
            name: None,
        });
        id
    }

    pub fn remove_guideline(&mut self, id: GuidelineId) -> bool {
        if let Some(pos) = self.guidelines.iter().position(|g| g.id == id) {
            self.guidelines.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn clear_guidelines(&mut self) {
        self.guidelines.clear();
    }

    pub fn active_page(&self) -> Option<&Page> {
        self.pages.get(self.active_page)
    }

    pub fn active_page_mut(&mut self) -> Option<&mut Page> {
        self.pages.get_mut(self.active_page)
    }

    pub fn find_shape(&self, id: ShapeId) -> Option<&Shape> {
        self.active_page()?.find_shape(id).map(|(s, _)| s)
    }

    pub fn find_shape_mut(&mut self, id: ShapeId) -> Option<&mut Shape> {
        self.active_page_mut()?.find_shape_mut(id).map(|(s, _)| s)
    }

    pub fn add_shape_to_active_layer(&mut self, shape: Shape) {
        if let Some(page) = self.active_page_mut() {
            if let Some(layer) = page.active_layer_mut() {
                layer.shapes.push(shape);
            }
        }
    }

    pub fn remove_shape(&mut self, id: ShapeId) -> Option<(Shape, usize)> {
        let page = self.active_page_mut()?;
        for (layer_idx, layer) in page.layers.iter_mut().enumerate() {
            if let Some(shape) = layer.remove_shape(id) {
                return Some((shape, layer_idx));
            }
        }
        None
    }

    /// Calculate bounding box covering all currently selected shapes.
    pub fn selection_bounds(&self) -> Option<Rect> {
        if self.selection.is_empty() {
            return None;
        }

        let mut bounds: Option<Rect> = None;
        for &id in &self.selection.ids {
            if let Some(shape) = self.find_shape(id) {
                if let Some(sb) = shape.world_bounds() {
                    bounds = match bounds {
                        Some(b) => Some(b.union(&sb)),
                        None => Some(sb),
                    };
                }
            }
        }
        bounds
    }

    /// Hit-test all visible and unlocked shapes in top-to-bottom order.
    pub fn hit_test(&self, world_pt: core_geom::Point2, tolerance: f64) -> Option<ShapeId> {
        let page = self.active_page()?;
        // Check top layers first, and inside each layer from last to first
        for layer in page.layers.iter().rev() {
            if !layer.visible || layer.locked {
                continue;
            }
            for shape in layer.shapes.iter().rev() {
                if !shape.visible || shape.locked {
                    continue;
                }
                if shape.hit_test(world_pt, tolerance) {
                    return Some(shape.id);
                }
            }
        }
        None
    }
}
