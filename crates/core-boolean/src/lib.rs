//! Boolean path operations for VDraw — stub for Phase 4.

use core_geom::Polycurve;

/// The type of boolean operation to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BooleanOp {
    Weld,
    Trim,
    Intersect,
    Simplify,
    FrontMinusBack,
    BackMinusFront,
    CreateBoundary,
    ExclusiveOr,
}

/// Perform a boolean operation on two paths.
/// Stub for Phase 4: returns a clone of `a` as placeholder.
pub fn boolean_op(op: BooleanOp, a: &Polycurve, _b: &Polycurve) -> Polycurve {
    let _ = op;
    a.clone()
}
