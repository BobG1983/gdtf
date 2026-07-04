//! The **area-damage-field runtime MECHANICS** (GTW-545, child GTW-41f; GTW-553 splits the
//! per-consequence behaviours out into the [`crate::effects::fields`] palette) — persistent
//! per-tile damage zones: the catalog model + two registries, the per-round [`tick_fields`]
//! clock that invokes the palette generically, and the [`FieldTicked`] signal the presenter
//! reads.
//!
//! An area-damage field is a persistent hazard on a `(cell, level)` — a toxic-waste pool, an
//! electrified floor, a patch of burning ground — that eats the [`Hp`](crate::ganger::Hp) of
//! whatever ganger stands on it, once per turn, until it expires
//! (`docs/combat/resolution.md` — the area-damage-field beat of GTW-41). The split follows the
//! GTW-550/552 palette line:
//!
//! - The MECHANICS live here: the catalog-authoring [`FieldDef`] (the `field` submodule — the
//!   flat authored RON surface, unchanged by GTW-553), the two registries (the `registry`
//!   submodule — the [`FieldDefRegistry`] CATALOG the folder loader builds, the
//!   [`ArmorRegistry`](crate::armor::ArmorRegistry) mirror, and the live [`FieldRegistry`]
//!   RESOURCE, the per-`(cell, level)` placed-field map keyed by
//!   [`CellLevel`](crate::metric::CellLevel), the [`CoverLedger`](crate::cover::CoverLedger)
//!   mirror — which exposes the [`FieldRegistry::spawn`] placement API the GTW-547 on-death
//!   `LeaveField` effect consumes), the situation seeding
//!   ([`setup_battle`](crate::situation::setup_battle)'s), and the CLOCK (the `tick` submodule).
//! - The per-consequence BEHAVIOURS — the per-turn flat drain, the whole-armor immunity gate,
//!   the Turns/Permanent duration — live in the GTW-553 [`crate::effects::fields`] palette
//!   (one self-contained file per consequence + the closed [`FieldEffect`] vocabulary),
//!   which the clock and the [`PlacedField`] lifetime invoke GENERICALLY through the shared
//!   [`ApplyFieldEffect`] trait — no per-consequence match lives on this side.
//!
//! [`tick_fields`] runs once per full round (the SAME enemy-phase-start cadence as the §9
//! bleed-out clock, [`enemy_phase_started`](crate::bleed::enemy_phase_started)): it reads each
//! fielded cell's occupant off the [`OccupancyGrid`](crate::occupancy::OccupancyGrid) and
//! drives the palette's exempt/drain verbs directly, then counts every placement's lifetime
//! down and removes the expired ones. [`FieldTicked`] is a buffered Bevy **message**
//! (`bevy-traps.md` #4 — NOT the observer `Event` API), mirroring
//! [`DotTicked`](crate::dot::DotTicked). Pure, render-free model logic: no renderer, no pixel;
//! the drain saturates (no underflow, no `unwrap`).

mod field;
mod registry;
mod tick;

#[cfg(test)]
mod test;

pub use field::FieldDef;
pub use registry::{FieldDefRegistry, FieldKey, FieldRegistry, PlacedField};
pub use tick::{FieldAfflicted, FieldOngoing, FieldTicked, tick_fields};

// The consequence payload newtypes live with their behaviours in the GTW-553 palette
// (`crate::effects::fields`); re-exported here so `crate::fields::*` and the crate-root
// flat `gdtf_battle_sim::*` paths keep resolving unchanged.
pub use crate::effects::fields::{
    ApplyFieldEffect, FieldDamage, FieldDuration, FieldEffect, FieldTurns, ImmuneArmorTypes,
};
