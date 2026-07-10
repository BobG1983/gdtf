//! The **area-damage fields** family — palette + mechanics in ONE home (GTW-545 mechanics,
//! child GTW-41f; GTW-553 palette; GTW-638 merged the runtime mechanics in from the
//! dissolved `acts_runtime`): persistent per-tile damage zones — the catalog model + two
//! registries, the per-round [`tick_fields`](crate::effects::fields::tick_fields) clock, and the closed [`FieldEffect`](crate::effects::fields::FieldEffect)
//! vocabulary with one isolated behaviour per consequence.
//!
//! An area-damage field is a persistent hazard on a `(cell, level)` — a toxic-waste pool, an
//! electrified floor, a patch of burning ground — that eats the [`Hp`](crate::ganger::Hp) of
//! whatever ganger stands on it, once per turn, until it expires
//! (`docs/combat/resolution.md` — the area-damage-field beat of GTW-41).
//!
//! ## The mechanics side (GTW-545)
//!
//! - The catalog-authoring [`FieldDef`](crate::effects::fields::FieldDef) (the `field` submodule — the flat authored RON
//!   surface, unchanged by GTW-553).
//! - The two registries (the `registry` submodule — the [`FieldDefRegistry`](crate::effects::fields::FieldDefRegistry) CATALOG the
//!   folder loader builds, the [`ArmorRegistry`](crate::armor::ArmorRegistry) mirror, and
//!   the live [`FieldRegistry`](crate::effects::fields::FieldRegistry) RESOURCE, the per-`(cell, level)` placed-field map keyed by
//!   [`CellLevel`](crate::metric::CellLevel), the [`CoverLedger`](crate::cover::CoverLedger)
//!   mirror — which exposes the [`FieldRegistry::spawn`](crate::effects::fields::FieldRegistry::spawn) placement API the GTW-547 on-death
//!   `LeaveField` effect consumes) and the situation seeding
//!   ([`setup_battle`](crate::situation::setup_battle)'s).
//! - The CLOCK (the `tick` submodule): [`tick_fields`](crate::effects::fields::tick_fields) runs once per full round (the SAME
//!   enemy-phase-start cadence as the §9 bleed-out clock,
//!   [`enemy_phase_started`](crate::effects::bleed::enemy_phase_started)) — it reads each
//!   fielded cell's occupant off the [`OccupancyGrid`](crate::occupancy::OccupancyGrid) and
//!   drives the palette's exempt/drain verbs directly, then counts every placement's
//!   lifetime down and removes the expired ones. [`FieldTicked`](crate::effects::fields::FieldTicked) is a buffered Bevy
//!   **message** (`bevy-traps.md` #4 — NOT the observer `Event` API), mirroring
//!   [`DotTicked`](crate::effects::dot::DotTicked).
//!
//! ## The palette side (GTW-553)
//!
//! - `apply_effect` — the shared [`ApplyFieldEffect`](crate::effects::fields::ApplyFieldEffect) trait (the palette contract: the
//!   exempt / drain / lifetime verbs, each defaulted to the inert answer) and the two
//!   borrowed occupant surfaces the clock lends for one fielded cell ([`OccupantArmor`](crate::effects::fields::OccupantArmor) —
//!   the worn-armor read the exemption gate inspects — and [`OccupantDrain`](crate::effects::fields::OccupantDrain) — the vitals +
//!   signal writers the drain mutates).
//! - `effect` — the closed [`FieldEffect`](crate::effects::fields::FieldEffect) vocabulary (the serde name↔type bridge, NOT yet
//!   RON-exposed — the authored surface stays the flat [`FieldDef`](crate::effects::fields::FieldDef) struct, projected into
//!   the vocabulary by [`FieldEffect::consequences_of`](crate::effects::fields::FieldEffect::consequences_of)) + its ONE thin delegation
//!   `impl ApplyFieldEffect` (every verb forwards through the one mechanical
//!   `with_behaviour` match — NO logic).
//! - ONE SELF-CONTAINED FILE PER CONSEQUENCE (`drain`, `immunity`, `duration`), each
//!   holding its payload newtype(s) + its isolated `ApplyX` struct + its
//!   `impl ApplyFieldEffect` + a `#[cfg(test)]` unit test.
//!
//! ## The discipline (the whole point)
//!
//! Adding a new consequence = ONE new per-consequence file + ONE [`FieldEffect`](crate::effects::fields::FieldEffect) variant +
//! ONE delegation arm (in `effect`) + ONE `mod` line here (plus its `consequences_of`
//! projection entry once the authored def carries it). No central logic `match`, no inline
//! tick branch, no authoring step scattered across the codebase. The mechanics NEVER match
//! on the consequence enum — the [`tick_fields`](crate::effects::fields::tick_fields) clock, the [`PlacedField`](crate::effects::fields::PlacedField) lifetime, the
//! [`FieldRegistry`](crate::effects::fields::FieldRegistry) placement state and the situation seeding invoke the shared
//! [`ApplyFieldEffect`](crate::effects::fields::ApplyFieldEffect) trait generically. The drain CADENCE stays the clock's (the
//! GTW-550/552 C3 mirror): the trait is invoked DIRECTLY on the tick path — never a
//! deferred command — so consequences resolve at exactly the same point of the round as
//! the pre-palette inline tick did. Pure, render-free model logic: no renderer, no pixel;
//! the drain saturates (no underflow, no `unwrap`).

mod apply_effect;
mod drain;
mod duration;
mod effect;
mod field;
mod immunity;
mod registry;
mod tick;

#[cfg(test)]
mod test;
#[cfg(test)]
mod test_lifetime;
#[cfg(test)]
mod test_turn_start;
#[cfg(test)]
mod tests;

pub use apply_effect::{ApplyFieldEffect, DrainExempt, FieldExpired, OccupantArmor, OccupantDrain};
pub use drain::{ApplyDrain, FieldDamage};
pub use duration::{ApplyDuration, FieldDuration, FieldTurns};
pub use effect::FieldEffect;
pub use field::FieldDef;
pub use immunity::{ApplyImmunity, ImmuneArmorTypes};
pub use registry::{FieldDefRegistry, FieldKey, FieldRegistry, PlacedField};
pub use tick::{FieldAfflicted, FieldOngoing, FieldTicked, tick_fields};
