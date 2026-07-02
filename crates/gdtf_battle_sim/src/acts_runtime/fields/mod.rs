//! The **area-damage-field runtime** (GTW-545, child GTW-41f) — persistent per-tile damage
//! zones: the model types + two registries, the per-round [`tick_fields`] drain, and the
//! [`FieldTicked`] signal the presenter reads.
//!
//! An area-damage field is a persistent hazard on a `(cell, level)` — a toxic-waste pool, an
//! electrified floor, a patch of burning ground — that eats the [`Hp`](crate::ganger::Hp) of
//! whatever ganger stands on it, once per turn, until it expires
//! (`docs/combat/resolution.md` — the area-damage-field beat of GTW-41). The model splits along
//! the model/runtime line, mirroring the GTW-544 DOT model and the GTW-9 §9 bleed clock:
//!
//! - The MODEL types live in the `field` submodule: the catalog-authoring [`FieldDef`] + its
//!   magnitude newtypes ([`FieldDamage`] / [`FieldTurns`]), the [`FieldDuration`] lifetime, and
//!   the whole-source-immunity [`ImmuneArmorTypes`] set.
//! - The two REGISTRIES live in the `registry` submodule: the [`FieldDefRegistry`] CATALOG (the name→def
//!   map the folder loader builds, the [`ArmorRegistry`](crate::armor::ArmorRegistry) mirror)
//!   and the live [`FieldRegistry`] RESOURCE (the per-`(cell, level)` placed-field map keyed by
//!   [`CellLevel`](crate::metric::CellLevel), the [`CoverLedger`](crate::cover::CoverLedger)
//!   mirror) — which exposes the [`FieldRegistry::spawn`] placement API the GTW-547 on-death
//!   `LeaveField` effect consumes.
//! - The CLOCK is [`tick_fields`]: once per full round (the SAME enemy-phase-start cadence as
//!   the §9 bleed-out clock, [`enemy_phase_started`](crate::bleed::enemy_phase_started)) it
//!   reads each fielded cell's occupant off the
//!   [`OccupancyGrid`](crate::occupancy::OccupancyGrid), drains its [`Hp`](crate::ganger::Hp)
//!   DIRECTLY by the field's per-turn damage — no armor matchup, no injury roll, no RNG, gated
//!   ONLY by whole-source armor-type immunity — emits a [`FieldTicked`] signal, flips the
//!   occupant to [`LifeState::Dead`](crate::ganger::LifeState::Dead) if the drain empties its
//!   HP (the GTW-544 DOT-kills precedent), then counts every `Turns` field down and removes the
//!   expired ones (a `Permanent` field never expires).
//!
//! [`FieldTicked`] is a buffered Bevy **message** (`bevy-traps.md` #4 — NOT the observer
//! `Event` API), mirroring [`DotTicked`](crate::dot::DotTicked). Pure, render-free model logic:
//! no renderer, no pixel; the drain saturates (no underflow, no `unwrap`).

mod field;
mod registry;
mod tick;

#[cfg(test)]
mod test;

pub use field::{FieldDamage, FieldDef, FieldDuration, FieldTurns, ImmuneArmorTypes};
pub use registry::{FieldDefRegistry, FieldKey, FieldRegistry, PlacedField};
pub use tick::{FieldTicked, tick_fields};
