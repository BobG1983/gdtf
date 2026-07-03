//! The **on-death effect palette** (GTW-552 — the on-death sibling of the GTW-558
//! attachment / GTW-550 injury palettes): the closed
//! [`OnDeathEffect`](crate::effects::on_death::OnDeathEffect) vocabulary + one isolated
//! behaviour per effect.
//!
//! ## The shape
//!
//! - `apply_effect` — the shared
//!   [`ApplyOnDeathEffect`](crate::effects::on_death::ApplyOnDeathEffect) trait (the
//!   palette contract: one `fan_at` verb), the borrowed
//!   [`DeathFanOut`](crate::effects::on_death::DeathFanOut) fan-out surface (the battle
//!   surfaces the resolver lends for one death, including its same-frame cascade
//!   work-queue), and the [`VictimRow`](crate::effects::on_death::VictimRow) victim-query
//!   row alias.
//! - `effect` — the closed serde [`OnDeathEffect`](crate::effects::on_death::OnDeathEffect)
//!   vocabulary (the RON name↔type bridge) + its ONE thin delegation
//!   `impl ApplyOnDeathEffect` (the single verb forwards through the one mechanical
//!   match — NO logic).
//! - ONE SELF-CONTAINED FILE PER EFFECT (`explode`, `leave_field`), each holding its
//!   payload newtype (where it has one) + its isolated `ApplyX` struct + its
//!   `impl ApplyOnDeathEffect` + a `#[cfg(test)]` unit test.
//!
//! ## The discipline (the whole point)
//!
//! Adding a new effect = ONE new per-effect file + ONE
//! [`OnDeathEffect`](crate::effects::on_death::OnDeathEffect) variant + ONE delegation arm
//! (in `effect`) + ONE `mod` line here. No central logic `match`, no per-variant folder-fn
//! in the resolver, no authoring step scattered across the codebase. The mechanics NEVER
//! match on the effect enum — the resolver
//! ([`resolve_on_death`](crate::on_death::resolve_on_death), whose cascade work-queue /
//! [`OnDeathOccurred`](crate::on_death::OnDeathOccurred) emission / heterogeneous World
//! access stay a MECHANICS concern under
//! [`acts_runtime::on_death`](crate::acts_runtime::on_death)) invokes the shared trait
//! generically.
//!
//! Dependency direction: this palette depends on the sim surfaces an effect fans into (the
//! GTW-541 [`aoe_affected`](crate::shot_pipeline::aoe::aoe_affected) geometry, the GTW-545
//! field registries, the occupancy / victim surfaces) and on the
//! [`OnDeathOccurred`](crate::on_death::OnDeathOccurred) work-item type; the on-death
//! MECHANICS (the resolver + its cascade fixpoint) depend on this palette. The cascade
//! CADENCE stays the resolver's (GTW-550 C3 mirror): the trait is invoked DIRECTLY on the
//! drain path — never a deferred command — so effects resolve at exactly the same point of
//! the cascade as the pre-palette folder functions did.

mod apply_effect;
mod effect;
mod explode;
mod leave_field;

#[cfg(test)]
mod tests;

pub use apply_effect::{ApplyOnDeathEffect, DeathFanOut, VictimRow};
pub use effect::OnDeathEffect;
pub use explode::{ApplyExplode, ExplodeDamage};
pub use leave_field::ApplyLeaveField;
