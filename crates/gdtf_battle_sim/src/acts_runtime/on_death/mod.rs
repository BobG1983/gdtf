//! The **on-death effects** runtime MECHANICS (GTW-547, child GTW-41g; GTW-552 splits the
//! effect behaviours out into the [`crate::effects::on_death`] palette) — the
//! [`OnDeathOccurred`] terminal-death signal, the [`OnDeath`] / [`CoverOnDeathRegistry`]
//! authoring carriers, and the [`resolve_on_death`] applier that fans each death's authored
//! [`OnDeathEffect`] generically through the palette's
//! [`ApplyOnDeathEffect`](crate::effects::on_death::ApplyOnDeathEffect) trait at the death
//! `(cell, level)`.
//!
//! The GTW-41 advanced-effects epic (child GTW-41g — this is a ticket-defined feature; the
//! on-death beat is not yet written into `docs/combat/resolution.md`): when a source dies —
//! a ganger reaches [`LifeState::Dead`](crate::ganger::LifeState) or a piece of cover is
//! destroyed — it may fan an authored effect. Two effects ship in the palette (adding one =
//! ONE palette file + ONE enum variant + ONE delegation arm + ONE `mod` line, all in
//! [`crate::effects::on_death`] — never a resolver change):
//!
//! - [`OnDeathEffect::Explode`] — fan a GTW-541
//!   [`aoe_affected`](crate::shot_pipeline::aoe::aoe_affected) blast at the death cell; every
//!   ganger in the radius takes a flat, deterministic, armor-bypassing, RNG-free
//!   [`Hp`](crate::ganger::Hp) drain (the [`tick_dot`](crate::dot::tick_dot) /
//!   [`tick_fields`](crate::fields::tick_fields) direct-drain model). A lethal blast KILLS +
//!   cascades.
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
//! module owns only the signal TYPE, the authoring carriers, and the resolver — the effect
//! BEHAVIOURS live in the palette.
//!
//! ## Module layout (code-health)
//!
//! `mod.rs` is WIRING-ONLY (submodule declarations + re-exports); the focused submodules are
//! the `signal` submodule ([`OnDeathOccurred`]), the `component` submodule ([`OnDeath`]), the
//! `resolve` submodule ([`resolve_on_death`] + [`CoverOnDeathRegistry`]), and a `tests`
//! submodule. The effect vocabulary ([`OnDeathEffect`]), its per-effect behaviours, and the
//! [`ExplodeDamage`] payload live in [`crate::effects::on_death`] and are re-exported below.
//!
//! Render-free, deterministic model logic: the blast drain takes NO RNG (the sorted
//! [`aoe_affected`](crate::shot_pipeline::aoe::aoe_affected) set + a flat drain), so a demo
//! explosion is byte-stable and does not perturb the seeded autobattle stream.

mod component;
mod resolve;
mod signal;

#[cfg(test)]
mod tests;

pub use component::OnDeath;
pub use resolve::{CoverOnDeathRegistry, resolve_on_death};
pub use signal::OnDeathOccurred;

// The effect vocabulary + per-effect behaviours live in the GTW-552 palette
// (`crate::effects::on_death`); re-exported here so `crate::on_death::*` and the crate-root
// flat `gdtf_battle_sim::*` paths keep resolving unchanged.
pub use crate::effects::on_death::{
    ApplyOnDeathEffect, DeathFanOut, ExplodeDamage, OnDeathEffect, VictimRow,
};
