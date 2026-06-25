//! Slab-HP ledger: the model's single authoritative store of floor/roof **slab**
//! structural HP, keyed `(cell, level)` — the GTW-365 slab mirror of the GTW-364
//! [`crate::cover`] ledger (user-ruled 2026-06-22).
//!
//! Per `docs/combat/resolution.md` §3.1, a slab is a physical object with its **own
//! armor stats + HP**, using the same armor/damage model as a ganger and cover (so
//! this module **reuses** [`crate::armor::ArmorProtection`] /
//! [`crate::armor::ArmorHardness`] — a slab is not a second armor model). A destroyed
//! slab stops blocking rounds AND passes line of sight (the round + the LOS march
//! already honor [`SlabState::Destroyed`](crate::surface::SlabState) — both fly
//! through), but it does **NOT** become walkable (movement / vertical links are out of
//! scope per the user ruling). The HP lives on **this** ledger; depletion to zero emits
//! a slab-destroyed event carrying `(cell, level)`, which the fire path bridges into a
//! buffered [`SlabDestroyed`](crate::occupancy_sync::SlabDestroyed) message — a
//! model-authoritative fact the view mirrors (ADR-0001,
//! `docs/decisions/0001-rust-bevy-rewrite.md`).
//!
//! Three design points carried from the cover precedent and the C7 ruling:
//!
//! 1. **One unified map.** [`SlabLedger`] keys [`CellLevel`](crate::metric::CellLevel)
//!    → [`SlabEntry`] for every slab — there is no second slab-HP structure anywhere
//!    in the sim.
//! 2. **Lazy seeding from TUNING.** `current_hp` is seeded to `max_hp` on **first
//!    access** from the [`SlabDefaults`](crate::tuning::SlabDefaults) combat-tuning
//!    leaf — NOT a per-piece authored value (slabs are uniform level structure,
//!    authored as a bare `(cell, level)` list with no per-slab HP). The tuning leaf is
//!    genuinely consumed by [`SlabLedger::prototype_for`] on the live depletion path
//!    (C7: no dead leaf).
//! 3. **Marker-only depletion.** [`SlabLedger::deplete_slab`] spends HP and, when
//!    `current_hp` reaches zero, sets [`SlabDestroyedFlag`] and returns a
//!    [`SlabEvent::Destroyed`] carrying the [`CellLevel`](crate::metric::CellLevel).
//!    The surface-grid `destroy_slab` + LOS rebuild are the fire path's bridge — this
//!    module emits the marker ONLY.

mod brace_stair_cells;
mod ledger;
mod types;

#[cfg(test)]
mod test;

pub use brace_stair_cells::BraceStairCells;
pub use ledger::SlabLedger;
pub use types::{SlabDamage, SlabDestroyedFlag, SlabEntry, SlabEvent, SlabHp};
