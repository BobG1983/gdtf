//! dependency direction: this overlay reads the AUTHORITATIVE sim
//! Unlike the DEBUG-only `reachable` overlay (which compiles only under
//! `#[cfg(debug_assertions)]`), the field overlay is a SHIPPING VIEW (the playability rule: a
//! damage zone MUST be visible), so it is NOT `#[cfg(debug_assertions)]`-gated. The overlay
mod overlay;

pub use overlay::{FieldCellSprite, draw_field_overlay};
