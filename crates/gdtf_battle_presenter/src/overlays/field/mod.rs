//! The area-damage-field overlay (GTW-545, child GTW-41f): the persistent per-cell VIEW that
//! draws the sim's live [`FieldRegistry`](gdtf_battle_sim::FieldRegistry) so a seeded field (a
//! toxic-waste pool, an electrified floor, a patch of burning ground) is VISIBLE on the
//! battlefield.
//!
//! Per ADR-0001 (the presenter owns ALL sim→view drawing) and the `input → presenter → sim`
//! dependency direction: this overlay reads the AUTHORITATIVE sim
//! [`FieldRegistry`](gdtf_battle_sim::FieldRegistry) resource DIRECTLY (a battle-lifetime
//! [`Resource`](bevy::prelude::Resource) `setup_battle` seeds) and DRAWS one translucent hazard
//! tile per fielded cell on the active storey — it never writes the sim. This is the SAME
//! one-way sim-read shape as the terrain draw reading the cover ledger.
//!
//! Unlike the DEBUG-only `reachable` overlay (which compiles only under
//! `#[cfg(debug_assertions)]`), the field overlay is a SHIPPING VIEW (the playability rule: a
//! damage zone MUST be visible), so it is NOT `#[cfg(debug_assertions)]`-gated. The overlay
//! follows `PageUp` with no extra wiring — the draw system reads
//! [`ActiveLevel`](crate::ActiveLevel) live and hard-cuts to the active storey.

mod overlay;

pub use overlay::{FieldCellSprite, draw_field_overlay};
