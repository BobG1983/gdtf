//! The E4.4 **ammo state + shared `can_fire` guard set** — the two pieces the HUD
//! fire button and the E4.5 `fire()` act both reason over.
//!
//! Two things land here (`docs/combat/resolution.md` §"What's pure math vs sim":
//! "ammo clamp"; §9 `can_stabilize` precedent: "button and act share one guard
//! set"):
//!
//! 1. The current-rounds AMMO state — a [`Magazine`] Component carrying the rounds
//!    currently loaded, **clamped at construction** by the weapon's
//!    [`MagazineSize`](crate::weapon::MagazineSize) (a weapon NUMBER; the current
//!    rounds are battle-local state). A **saturating** per-round decrement
//!    ([`Magazine::spend_round`], floors at `0`, never underflows) and the
//!    burst-clamp primitive [`clamp_burst`] (the round count `fire()` may actually
//!    loop = `min(mode shots, rounds left)`). **Reload boundary:** this slice ships
//!    ONLY the ammo state + the per-round decrement + the burst clamp — a standalone
//!    `reload()` act with the `reload_tu` refill (resolution.md L166 names
//!    `reload_tu` tunable, but no `reload_tu` tuning leaf exists yet and a reload
//!    ACT is not in GTW-9's scope) is OUT of E4, noted here, not built.
//!
//! 2. The [`can_fire`] GUARD SET — the validation the HUD button and `fire()`
//!    SHARE (one guard set, the §9 `can_stabilize` precedent). It returns `true`
//!    iff ALL hold: the shooter is [`LifeState::Alive`](crate::ganger::LifeState::Alive);
//!    affords the selected mode's TU charge ([`mode_tu_cost`], checked via E4.0
//!    [`can_spend_tu`](crate::tu::can_spend_tu)); has ammo
//!    ([`Magazine`] rounds ≥ 1); and the target `(cell, level)` is
//!    [`in_bounds`]. **LOS/fog is explicitly NOT a `can_fire` input** — the
//!    has-LOS / fog gate is PLAYER POLICY in the presenter (resolution.md §"What's
//!    pure math vs sim": fog "never enters the shared act"); `can_fire` validates
//!    only alive + TU + ammo + in-bounds, taking no `has_los`/visibility argument.
//!
//! The TU charge ([`mode_tu_cost`]) is the **single source** both `can_fire` and
//! the E4.5 `fire()` debit read, so the affordability check and the actual charge
//! can never diverge. Pure math, no [`World`](bevy::ecs::world::World) access —
//! render-free, **zero pixels**.
//!
//! GTW-201 code-health: this concern is a dir-module split by responsibility — the
//! ammo state ([`ammo`]: the [`Magazine`] + the [`clamp_burst`] primitive) and the
//! shared firing guard ([`guard`]: [`mode_tu_cost`] / [`in_bounds`] / [`FireActor`]
//! / [`can_fire`]). This `mod.rs` is wiring-only; every public path is preserved
//! via the re-exports below.

mod ammo;
mod guard;

#[cfg(test)]
mod test;

pub use ammo::{Magazine, clamp_burst};
pub use guard::{FireActor, can_fire, in_bounds, mode_tu_cost};
