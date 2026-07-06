//! The **on-death effects** family — palette + mechanics in ONE home (GTW-547 mechanics,
//! child GTW-41g; GTW-552 palette; GTW-638 merged the runtime mechanics in from the
//! dissolved `acts_runtime`): the [`OnDeathOccurred`](crate::effects::on_death::OnDeathOccurred) terminal-death signal, the
//! [`OnDeath`](crate::effects::on_death::OnDeath) / [`CoverOnDeathRegistry`](crate::effects::on_death::CoverOnDeathRegistry) authoring carriers, the [`resolve_on_death`](crate::effects::on_death::resolve_on_death)
//! applier, and the closed [`OnDeathEffect`](crate::effects::on_death::OnDeathEffect) vocabulary with one isolated behaviour per
//! effect.
//!
//! The GTW-41 advanced-effects epic (child GTW-41g — this is a ticket-defined feature; the
//! on-death beat is not yet written into `docs/combat/resolution.md`): when a source dies —
//! a ganger reaches [`LifeState::Dead`](crate::ganger::LifeState) or a piece of cover is
//! destroyed — it may fan an authored effect. Two effects ship:
//!
//! - [`OnDeathEffect::Explode`](crate::effects::on_death::OnDeathEffect::Explode) — fan a GTW-541
//!   [`aoe_affected`](crate::shot_pipeline::aoe::aoe_affected) blast at the death cell; every
//!   ganger in the radius takes a flat, deterministic, armor-bypassing, RNG-free
//!   [`Hp`](crate::ganger::Hp) drain (the [`tick_dot`](crate::effects::dot::tick_dot) /
//!   [`tick_fields`](crate::effects::fields::tick_fields) direct-drain model). A lethal
//!   blast KILLS + cascades.
//! - [`OnDeathEffect::LeaveField`](crate::effects::on_death::OnDeathEffect::LeaveField) — spawn a GTW-545 field
//!   ([`FieldRegistry::spawn`](crate::effects::fields::FieldRegistry::spawn)) at the death
//!   cell so it becomes a live, persistent hazard.
//!
//! ## Emission — EVERY terminal death / cover-destroyed gate
//!
//! [`OnDeathOccurred`](crate::effects::on_death::OnDeathOccurred) is emitted from every terminal gate the sim reaches (so a death by ANY
//! path fans its effect): the ranged fire dispatch (primary + splash), the melee ganger kill,
//! the falls kill, the per-round bleed / DOT / area-damage-field clocks, and both cover-destroy
//! sites (ranged + melee). The scattered emit sites live with each producing system; this
//! module owns the signal TYPE, the authoring carriers, the resolver, AND the effect
//! behaviours.
//!
//! ## The mechanics side (GTW-547)
//!
//! The `signal` submodule ([`OnDeathOccurred`](crate::effects::on_death::OnDeathOccurred)), the `component` submodule ([`OnDeath`](crate::effects::on_death::OnDeath)),
//! and the `resolve` submodule ([`resolve_on_death`](crate::effects::on_death::resolve_on_death) + [`CoverOnDeathRegistry`](crate::effects::on_death::CoverOnDeathRegistry)) — plus the
//! `test` submodule (the mechanics suite). Render-free, deterministic model logic: the
//! blast drain takes NO RNG (the sorted
//! [`aoe_affected`](crate::shot_pipeline::aoe::aoe_affected) set + a flat drain), so a demo
//! explosion is byte-stable and does not perturb the seeded autobattle stream.
//!
//! ## The palette side (GTW-552)
//!
//! - `apply_effect` — the shared [`ApplyOnDeathEffect`](crate::effects::on_death::ApplyOnDeathEffect) trait (the palette contract: one
//!   `fan_at` verb), the borrowed [`DeathFanOut`](crate::effects::on_death::DeathFanOut) fan-out surface (the battle surfaces the
//!   resolver lends for one death, including its same-frame cascade work-queue), and the
//!   [`VictimRow`](crate::effects::on_death::VictimRow) victim-query row alias.
//! - `effect` — the closed serde [`OnDeathEffect`](crate::effects::on_death::OnDeathEffect) vocabulary (the RON name↔type bridge) +
//!   its ONE thin delegation `impl ApplyOnDeathEffect` (the single verb forwards through
//!   the one mechanical match — NO logic).
//! - ONE SELF-CONTAINED FILE PER EFFECT (`explode`, `leave_field`), each holding its
//!   payload newtype (where it has one) + its isolated `ApplyX` struct + its
//!   `impl ApplyOnDeathEffect` + a `#[cfg(test)]` unit test.
//!
//! ## The discipline (the whole point)
//!
//! Adding a new effect = ONE new per-effect file + ONE [`OnDeathEffect`](crate::effects::on_death::OnDeathEffect) variant + ONE
//! delegation arm (in `effect`) + ONE `mod` line here. No central logic `match`, no
//! per-variant folder-fn in the resolver, no authoring step scattered across the codebase.
//! The mechanics NEVER match on the effect enum — the resolver ([`resolve_on_death`](crate::effects::on_death::resolve_on_death),
//! whose cascade work-queue / [`OnDeathOccurred`](crate::effects::on_death::OnDeathOccurred) emission / heterogeneous World access
//! stay a MECHANICS concern) invokes the shared trait generically. The cascade CADENCE
//! stays the resolver's (GTW-550 C3 mirror): the trait is invoked DIRECTLY on the drain
//! path — never a deferred command — so effects resolve at exactly the same point of the
//! cascade as the pre-palette folder functions did.

mod apply_effect;
mod component;
mod effect;
mod explode;
mod leave_field;
mod resolve;
mod signal;

#[cfg(test)]
mod test;
#[cfg(test)]
mod tests;

pub use apply_effect::{ApplyOnDeathEffect, DeathFanOut, VictimRow};
pub use component::OnDeath;
pub use effect::OnDeathEffect;
pub use explode::{ApplyExplode, ExplodeDamage};
pub use leave_field::ApplyLeaveField;
pub use resolve::{CoverOnDeathRegistry, resolve_on_death};
pub use signal::OnDeathOccurred;
