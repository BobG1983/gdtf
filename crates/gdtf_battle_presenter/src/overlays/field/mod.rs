//! Field damage overlay over authoritative sim cells.
//! Shipping view (not debug-gated): active damage zones stay visible.
mod overlay;

pub use overlay::{FieldCellSprite, draw_field_overlay};
