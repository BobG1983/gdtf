//! Drive a type's `Deserialize` impl until every enum body has been seen.

use serde::de::DeserializeOwned;

use super::{
    doc::ShapeDoc,
    fault::{ShapeFault, ShapeFaultKind, TracePassBudget, TracedType},
    recorder::ShapeRecorder,
    tracer::ShapeTracer,
};

/// How many walks over one type a trace may take before giving up.
pub const SHAPE_PASS_BUDGET: TracePassBudget = TracePassBudget::new(4096);

/// Text published when a type's shape cannot be traced.
pub const FALLBACK_SHAPE_TEXT: &str = "(root:Unit,defs:[])";

/// Trace `T`'s RON shape out of its own `Deserialize` impl.
///
/// # Errors
///
/// Fails when `T` contains itself, asks for `deserialize_any`, or does not settle in
/// [`SHAPE_PASS_BUDGET`] passes.
pub fn shape_trace<T: DeserializeOwned>() -> Result<ShapeDoc, ShapeFault> {
    let subject = TracedType::new(core::any::type_name::<T>());
    let mut tracer = ShapeTracer::new();
    for _ in 0..*SHAPE_PASS_BUDGET {
        if let Err(fault) = T::deserialize(ShapeRecorder::new(&mut tracer)) {
            return Err(fault.traced_from(subject));
        }
        let root = tracer
            .take_last()
            .map_err(|fault| fault.traced_from(subject))?;
        if tracer.is_settled() {
            return Ok(tracer.finish(root));
        }
    }
    Err(ShapeFault::new(ShapeFaultKind::PassBudget(SHAPE_PASS_BUDGET)).traced_from(subject))
}

/// Traced RON shape text for `T`, falling back to [`FALLBACK_SHAPE_TEXT`].
#[must_use]
pub fn shape_text<T: DeserializeOwned>() -> String {
    shape_trace::<T>()
        .ok()
        .and_then(|doc| ron::ser::to_string(&doc).ok())
        .unwrap_or_else(|| FALLBACK_SHAPE_TEXT.to_owned())
}
