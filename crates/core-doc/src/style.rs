use crate::color::Color;
use serde::{Deserialize, Serialize};

/// Line cap style for open path endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum LineCap {
    #[default]
    Butt,
    Round,
    Square,
}

/// Line join style for path vertices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum LineJoin {
    #[default]
    Miter,
    Round,
    Bevel,
}

/// Stroke appearance properties.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stroke {
    pub color: Color,
    /// Width in millimeters.
    pub width: f64,
    /// Whether stroke qualifies as a laser hairline (<= 0.1 mm).
    pub hairline: bool,
    pub cap: LineCap,
    pub join: LineJoin,
    pub miter_limit: f64,
}

impl Default for Stroke {
    fn default() -> Self {
        Self {
            color: Color::BLACK,
            width: 0.2, // 0.2 mm default
            hairline: false,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            miter_limit: 4.0,
        }
    }
}

impl Stroke {
    pub fn new(color: Color, width: f64) -> Self {
        Self {
            color,
            width,
            hairline: width <= 0.1,
            ..Default::default()
        }
    }

    pub fn hairline(color: Color) -> Self {
        Self {
            color,
            width: 0.05,
            hairline: true,
            ..Default::default()
        }
    }
}

/// Styling configuration for vector shapes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Style {
    pub fill: Option<Color>,
    pub stroke: Option<Stroke>,
}

impl Style {
    pub fn new(fill: Option<Color>, stroke: Option<Stroke>) -> Self {
        Self { fill, stroke }
    }

    pub fn default_outline() -> Self {
        Self {
            fill: None,
            stroke: Some(Stroke::default()),
        }
    }

    pub fn default_filled(fill_color: Color) -> Self {
        Self {
            fill: Some(fill_color),
            stroke: Some(Stroke::default()),
        }
    }

    pub fn laser_cut() -> Self {
        Self {
            fill: None,
            stroke: Some(Stroke::hairline(Color::RED)),
        }
    }

    pub fn laser_score() -> Self {
        Self {
            fill: None,
            stroke: Some(Stroke::hairline(Color::BLUE)),
        }
    }
}
