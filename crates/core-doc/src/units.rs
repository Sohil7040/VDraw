use serde::{Deserialize, Serialize};

/// Document measurement units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Unit {
    #[default]
    Millimeter,
    Inch,
    Point,
}

impl Unit {
    pub const MM_PER_INCH: f64 = 25.4;
    pub const PT_PER_INCH: f64 = 72.0;
    pub const MM_PER_PT: f64 = 25.4 / 72.0;

    pub fn label(self) -> &'static str {
        match self {
            Unit::Millimeter => "mm",
            Unit::Inch => "in",
            Unit::Point => "pt",
        }
    }

    /// Convert a value in this unit to millimeters.
    pub fn to_mm(self, value: f64) -> f64 {
        match self {
            Unit::Millimeter => value,
            Unit::Inch => value * Self::MM_PER_INCH,
            Unit::Point => value * Self::MM_PER_PT,
        }
    }

    /// Convert a millimeter value into this unit.
    pub fn from_mm(self, mm: f64) -> f64 {
        match self {
            Unit::Millimeter => mm,
            Unit::Inch => mm / Self::MM_PER_INCH,
            Unit::Point => mm / Self::MM_PER_PT,
        }
    }

    /// Convert a value in this unit to screen pixels at the given DPI.
    pub fn to_pixels(self, value: f64, dpi: f64) -> f64 {
        let mm = self.to_mm(value);
        mm * (dpi / Self::MM_PER_INCH)
    }

    /// Convert screen pixels at given DPI into this unit.
    pub fn from_pixels(self, px: f64, dpi: f64) -> f64 {
        let mm = px * (Self::MM_PER_INCH / dpi);
        self.from_mm(mm)
    }
}
