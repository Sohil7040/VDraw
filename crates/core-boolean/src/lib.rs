//! Boolean, Knife, Eraser, Virtual Segment Delete, Offset, and Text-to-Path engine
//! for VDraw in pure Rust.

pub mod boolean;
pub mod command;
pub mod contracts;
pub mod convert;
pub mod eraser;
pub mod error;
pub mod knife;
pub mod offset;
pub mod text_to_path;
pub mod virtual_segment;

pub use boolean::{boolean_op, execute_boolean, simplify_shape, BooleanOp};
pub use command::{
    apply_boolean_to_document, apply_contour_to_document, apply_eraser_to_document,
    apply_knife_to_document, apply_text_to_document, apply_virtual_segment_delete_to_document,
    BooleanReplaceCommand,
};
pub use contracts::{boolean_with_options, BoolCancelToken, BoolError, BoolOp, BoolOptions};
pub use eraser::erase_shape;
pub use error::{BooleanError, TextError};
pub use knife::slice_shape;
pub use offset::{offset_shape, ContourJoin};
pub use text_to_path::{
    default_text_to_polycurve, text_to_polycurve, TextOptions, DEFAULT_FONT_BYTES,
};
pub use virtual_segment::{delete_virtual_segment, find_virtual_segments, VirtualSegment};
