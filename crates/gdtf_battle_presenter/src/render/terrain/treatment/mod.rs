//! The ONE shared storey-treatment classification (GTW-594 C1): every "which storeys
//! draw, and how" decision — the terrain draw band, the destruction-swap gates, the
//! ganger-visibility band fact, and the editor's preview band — resolves through
//! [`storey_treatment`], never a parallel rule.

mod classify;

#[cfg(test)]
mod test;

pub use classify::{ContextDepth, IsolateView, StoreyTreatment, StoreyViewMode, storey_treatment};
