//! RON shape documents traced out of a type's own `Deserialize` impl.

mod access;
/// The vocabulary a shape document is written in.
pub mod doc;
/// Why a shape trace could not finish.
pub mod fault;
/// Type and field names inside a shape document.
pub mod names;
mod recorder;
/// Published argument and reply shape text.
pub mod schema;
/// Trace entry points.
pub mod trace;
mod tracer;

pub use doc::{RonShape, ShapeBody, ShapeDef, ShapeDoc, ShapeField, ShapeVariant, VariantShape};
pub use fault::{FaultDetail, ShapeFault, ShapeFaultKind, TracePassBudget, TracedType};
pub use names::{ShapeFieldName, ShapeName};
pub use schema::{ArgSchemaRon, ReplySchemaRon};
pub use trace::{FALLBACK_SHAPE_TEXT, SHAPE_PASS_BUDGET, shape_text, shape_trace};

#[cfg(test)]
mod test;
