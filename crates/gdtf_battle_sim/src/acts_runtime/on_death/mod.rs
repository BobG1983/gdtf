//! The **on-death effects** runtime (GTW-547, child GTW-41g) — the [`OnDeathOccurred`]
//! terminal-death signal, the closed [`OnDeathEffect`] enum (RON-authored per weapon / gear /
//! cover), and the [`resolve_on_death`] applier that fans each variant's per-variant folder
//! function at the death `(cell, level)`.
//!
//! The GTW-41 advanced-effects epic (child GTW-41g — this is a ticket-defined feature; the
//! on-death beat is not yet written into `docs/combat/resolution.md`): when a source dies —
//! a ganger reaches [`LifeState::Dead`](crate::ganger::LifeState) or a piece of cover is
//! destroyed — it may fan an authored effect. Two variants ship (the enum is CLOSED but
//! designed to be extended: add a variant + a folder function = done):
//!
//! - [`OnDeathEffect::Explode`] — fan a GTW-541
//!   [`aoe_affected`](crate::shot_pipeline::aoe::aoe_affected) blast at the death cell; every ganger
//!   in the radius takes a flat, deterministic, armor-bypassing, RNG-free [`Hp`](crate::ganger::Hp)
//!   drain (the [`tick_dot`](crate::dot::tick_dot) / [`tick_fields`](crate::fields::tick_fields)
//!   direct-drain model). A lethal blast KILLS + cascades.
//! - [`OnDeathEffect::LeaveField`] — spawn a GTW-545 field
//!   ([`FieldRegistry::spawn`](crate::fields::FieldRegistry::spawn)) at the death cell so it
//!   becomes a live, persistent hazard.
//!
//! ## Emission — EVERY terminal death / cover-destroyed gate
//!
//! [`OnDeathOccurred`] is emitted from every terminal gate the sim reaches (so a death by ANY
//! path fans its effect): the ranged fire dispatch (primary + splash), the melee ganger kill,
//! the falls kill, the per-round bleed / DOT / area-damage-field clocks, and both cover-destroy
//! sites (ranged + melee). The scattered emit sites live with each producing system; this
//! module owns only the signal TYPE, the effect enum, and the resolver.
//!
//! ## Module layout (code-health)
//!
//! `mod.rs` is WIRING-ONLY (submodule declarations + re-exports); the focused submodules are the
//! `signal` submodule ([`OnDeathOccurred`]), the `effect` submodule ([`OnDeathEffect`] +
//! [`OnDeath`] + [`ExplodeDamage`]), the `resolve` submodule ([`resolve_on_death`] +
//! [`CoverOnDeathRegistry`] + the per-variant folder functions), and a `tests` submodule.
//!
//! Render-free, deterministic model logic: the blast drain takes NO RNG (the sorted
//! [`aoe_affected`](crate::shot_pipeline::aoe::aoe_affected) set + a flat drain), so a demo explosion
//! is byte-stable and does not perturb the seeded autobattle stream.

mod effect;
mod resolve;
mod signal;

#[cfg(test)]
mod tests;

pub use effect::{ExplodeDamage, OnDeath, OnDeathEffect};
pub use resolve::{CoverOnDeathRegistry, resolve_on_death};
pub use signal::OnDeathOccurred;
