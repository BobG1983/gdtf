//! The **field-consequence palette** (GTW-553 — the area-damage-field sibling of the
//! GTW-558 attachment / GTW-550 injury / GTW-552 on-death palettes): the closed
//! [`FieldEffect`](crate::effects::fields::FieldEffect) vocabulary + one isolated
//! behaviour per consequence.
//!
//! ## The shape
//!
//! - `apply_effect` — the shared
//!   [`ApplyFieldEffect`](crate::effects::fields::ApplyFieldEffect) trait (the palette
//!   contract: the exempt / drain / lifetime verbs, each defaulted to the inert answer)
//!   and the two borrowed occupant surfaces the clock lends for one fielded cell
//!   ([`OccupantArmor`](crate::effects::fields::OccupantArmor) — the worn-armor read the
//!   exemption gate inspects — and [`OccupantDrain`](crate::effects::fields::OccupantDrain)
//!   — the vitals + signal writers the drain mutates).
//! - `effect` — the closed [`FieldEffect`](crate::effects::fields::FieldEffect)
//!   vocabulary (the serde name↔type bridge, NOT yet RON-exposed — the authored surface
//!   stays the flat [`FieldDef`](crate::fields::FieldDef) struct, projected into the
//!   vocabulary by
//!   [`FieldEffect::consequences_of`](crate::effects::fields::FieldEffect::consequences_of))
//!   + its ONE thin delegation `impl ApplyFieldEffect` (every verb forwards through the
//!   one mechanical `with_behaviour` match — NO logic).
//! - ONE SELF-CONTAINED FILE PER CONSEQUENCE (`drain`, `immunity`, `duration`), each
//!   holding its payload newtype(s) + its isolated `ApplyX` struct + its
//!   `impl ApplyFieldEffect` + a `#[cfg(test)]` unit test.
//!
//! ## The discipline (the whole point)
//!
//! Adding a new consequence = ONE new per-consequence file + ONE
//! [`FieldEffect`](crate::effects::fields::FieldEffect) variant + ONE delegation arm (in
//! `effect`) + ONE `mod` line here (plus its `consequences_of` projection entry once the
//! authored def carries it). No central logic `match`, no inline tick branch, no
//! authoring step scattered across the codebase. The mechanics NEVER match on the
//! consequence enum — the fields MECHANICS (the
//! [`tick_fields`](crate::fields::tick_fields) clock, the
//! [`PlacedField`](crate::fields::PlacedField) lifetime, the
//! [`FieldRegistry`](crate::fields::FieldRegistry) placement state and the situation
//! seeding, all still under [`acts_runtime::fields`](crate::acts_runtime::fields)) invoke
//! the shared trait generically.
//!
//! Dependency direction: this palette depends on the sim surfaces a consequence touches
//! (the worn-armor relationship, the ganger vitals, the
//! [`FieldTicked`](crate::fields::FieldTicked) /
//! [`OnDeathOccurred`](crate::on_death::OnDeathOccurred) signals); the fields MECHANICS
//! depend on this palette. The drain CADENCE stays the clock's (the GTW-550/552 C3
//! mirror): the trait is invoked DIRECTLY on the tick path — never a deferred command —
//! so consequences resolve at exactly the same point of the round as the pre-palette
//! inline tick did.

mod apply_effect;
mod drain;
mod duration;
mod effect;
mod immunity;

#[cfg(test)]
mod tests;

pub use apply_effect::{ApplyFieldEffect, OccupantArmor, OccupantDrain};
pub use drain::{ApplyDrain, FieldDamage};
pub use duration::{ApplyDuration, FieldDuration, FieldTurns};
pub use effect::FieldEffect;
pub use immunity::{ApplyImmunity, ImmuneArmorTypes};
