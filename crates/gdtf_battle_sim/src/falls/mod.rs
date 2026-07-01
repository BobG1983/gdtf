//! The **fall mechanic** (GTW-523, child of GTW-39, parent GTW-17) — the authoritative
//! sim-side reaction to trigger (a): a floor **slab destroyed under a standing ganger**.
//!
//! `docs/combat/resolution.md` §Falls / `docs/combat/combat.md` (fall damage is now
//! in-scope): when a slab keyed `(cell, N)` — the FLOOR of storey `N` per the march boundary
//! convention (`shot_pipeline::march`: the slab between two storeys is keyed at the UPPER
//! level) — is destroyed, any LIVE ganger STANDING on it (`Position.level == N`, NOT `N + 1`
//! which is the ROOF) FALLS to the highest supported storey below and takes weight-free,
//! LINEAR fall damage routed through the SAME resolve/apply + injury pipeline a shot takes.
//!
//! This module is render-free, deterministic, and unit-testable with injected seeded RNG. Its
//! pieces (GTW-201 code-health: a directory module, `mod.rs` wiring-only + focused submodules
//! by concern):
//!
//! - [`message`] — the [`FallOccurred`] buffered output signal + its [`StoreysFallen`]
//!   distance newtype (the presenter's fall FX / log reader drains it).
//! - [`resolve`] — the pure [`resolve_drop`] verb (C2): scan a cell's slab column downward
//!   from a faller's start storey for the highest SUPPORTED storey (ground, or a `Present`
//!   slab), returning the [`DropLanding`] (landing storey + storeys fallen).
//! - [`damage`] — the pure `resolve_fall_hit` fork (C4 / C5): the weight-free
//!   `per_storey_damage × storeys` blow synthesized as a [`Matchup::Neutral`](crate::matchup::Matchup)
//!   kinetic hit through the SHARED
//!   [`synthesize_wound`](crate::resolve_and_apply::synthesize_wound) core the ganger path
//!   also uses (`resolve_hit` → `roll_severity`, one [`SeverityRng`](crate::rng::SeverityRng)
//!   draw → `apply_hit` → `roll_injury`, one [`InjuryRng`](crate::rng::InjuryRng) draw on a
//!   non-graze / non-fatal wound) — REUSING every landed combat-math verb, NO new RNG stream,
//!   NO `FightRng` draw (a fall has no attacker).
//! - [`system`] — the [`apply_falls`] ECS system (C1 / C2 / C3 / C6 / C7): reads the buffered
//!   [`SlabDestroyed`](crate::occupancy_sync::SlabDestroyed), applies the faller predicate +
//!   stair brace + drop resolution + damage synthesis, rewrites [`Position`](crate::ganger::Position)
//!   as ONE involuntary write (letting `Changed<Position>` drive the occupancy teardown), and
//!   emits [`FallOccurred`] + the EXISTING [`InjuryInflicted`](crate::acts::InjuryInflicted).
//! - [`plugin`] — the [`FallsPlugin`] (C7): registers the `FallOccurred` buffer and wires
//!   `apply_falls` `.in_set(SimSystems::Simulate)` with the EXPLICIT ordering
//!   `.after(dispatch_fire)` + `.after(sync_destroyed_slab)`.

mod damage;
mod message;
mod plugin;
mod resolve;
mod system;

#[cfg(test)]
mod tests;

pub use message::{FallOccurred, StoreysFallen};
pub use plugin::FallsPlugin;
pub use resolve::{DropLanding, resolve_drop};
pub use system::apply_falls;
