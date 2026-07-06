//! The GTW-306 3-FRAME ANIMATED impact FX — FX-B's slice.
//!
//! This module is the stub FX-A created so FX-B fills only the body (no `mod.rs`
//! edit collision): FX-A already declares + registers [`animate_impact`](animate::animate_impact)
//! in [`fx::mod`](super) and wires it into the `PresenterSystems::Overlay` band, and
//! defines the [`PendingImpact`](super::projectile::PendingImpact) SEAM
//! [`advance_projectiles`](super::projectile::advance_projectiles) spawns at a
//! projectile's arrival point (carrying the arrival world position + the shot's
//! [`DamageType`](gdtf_battle_sim::DamageType)).
//!
//! FX-B's job (this module): turn each arrived [`PendingImpact`](super::projectile::PendingImpact)
//! into a 3-FRAME
//! impact ANIMATION at its [`at`](super::projectile::PendingImpact::at) point —
//! play the damage type's three impact tiles
//! ([`EffectRoles::fx_for`](super::roles::EffectRoles::fx_for)`(damage).impact`,
//! the solid-burst → open-ring → breaking-ring sequence the sheet authors in row
//! cols 8,9,10) in order, each held for a few frames, then despawn. The muzzle
//! flash + traveling projectile stay sane in their own systems (FX-A); this slice
//! ONLY animates the impact.
//!
//! Pure VIEW (ADR-0001): it READS the [`PendingImpact`](super::projectile::PendingImpact) seam +
//! the data-driven
//! [`EffectRoles`](super::roles::EffectRoles) table and draws sprites; it never writes the sim.
//! Param-only throughout (`bevy-traps.md` #7).

mod animate;
mod animation;
mod signal;

#[cfg(test)]
mod test;

pub use animate::animate_impact;
// `pub(super)` (not `pub`): fx/mod.rs re-exports only `ShotImpactResolved` /
// `animate_impact` from this module, so a plain `pub` re-export here would trip the
// workspace's denied `unreachable_pub` — the fx-layer scope preserves the old
// `fx::impact::ImpactAnimation` path (the tuning-doc link + the sibling `animate` use).
pub(super) use animation::ImpactAnimation;
pub use signal::ShotImpactResolved;
