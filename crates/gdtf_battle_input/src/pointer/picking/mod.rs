//! Cursor->cell picking + the hover-highlight emitter (GTW-221 / GTW-251 / GTW-259 / GTW-300):
//! the [`InspectTarget`] resource (its LIVE hovered cell + an optional pinned target), the
//! [`pick_hovered_cell`] system that writes the hovered cell from the active pointer's cursor,
//! the [`world_to_cell`] inverse projection it uses, and the [`emit_highlight_request`] emitter
//! that drives the presenter's reticle.

mod highlight;
mod hovered;
mod projection;

#[cfg(test)]
mod test;

pub use highlight::emit_highlight_request;
pub use hovered::{InspectMode, InspectTarget, pick_hovered_cell};
pub use projection::world_to_cell;
