//! The GTW-306 traveling DIRECTIONAL projectile FX — the muzzle→target flight that
//! REPLACES GTW-290's smeared stretched-sprite "tracer".
//!
//! On each [`ShotFired`](gdtf_battle_sim::shot_fired::ShotFired) round the sim emits,
//! [`spawn_shot_projectiles`](spawn::spawn_shot_projectiles) spawns ONE small (one-tile)
//! projectile sprite at the
//! muzzle world point, showing the per-damage-type directional tile picked for the
//! shot's heading (the [`EffectRoles`](super::roles::EffectRoles) color row → its 8-way rose →
//! [`nearest_direction_index`](super::roles::nearest_direction_index) of the
//! trajectory).
//!
//! The bolt flies at a CONSTANT VELOCITY (the hot-reloadable
//! [`ProjectileVelocity`](super::tuning::ProjectileVelocity), px/sec) — not over a
//! fixed-seconds window — so every shot shares one visual SPEED regardless of how far
//! it travels; a long shot simply spends more frames in flight.
//! [`advance_projectiles`](advance::advance_projectiles)
//! steps the sprite `velocity × delta` along the muzzle→target ray each frame, the
//! sprite TRAVELS at constant scale, NEVER stretched/scaled along the vector (that smear
//! was the GTW-290 bug). A straight shot is one direction, so it shows one tile for the
//! whole flight, and DESPAWNS the instant it reaches the target point.
//!
//! Where the bolt flies depends on WHAT the round struck
//! ([`ShotFired::kind`](gdtf_battle_sim::shot_fired::ShotFired)): for a
//! [`Ganger`](gdtf_battle_sim::resolve_coarse::ShotKind::Ganger) hit it flies to that hit entity's
//! CURRENT rendered world position (its presenter [`Transform`](bevy::prelude::Transform),
//! looked up through
//! [`GangerSprites`](crate::GangerSprites)) — which already reflects the target's stance
//! / silhouette height, so the bolt angles correctly toward a prone / kneeling target
//! with NO sim change and NO 3D impact field. For a non-ganger hit or a clean miss it
//! flies to the impact `(cell, level)` ([`cell_to_world`](crate::cell_to_world) of
//! `msg.impact_cell` / `msg.impact_level`). When the bolt REACHES that target point it
//! despawns and spawns the impact there.
//!
//! On arrival, [`advance_projectiles`](advance::advance_projectiles) spawns a
//! [`PendingImpact`](pending::PendingImpact) at the arrival
//! point carrying the shot's [`DamageType`](gdtf_battle_sim::weapon::DamageType) — the SEAM
//! FX-B's [`animate_impact`](super::impact::animate_impact) reads to play the
//! damage type's 3-frame impact animation there. FX-A only HANDS OFF the impact
//! position + type; FX-B owns the animation.
//!
//! A burst / full-auto shot emits one [`ShotFired`](gdtf_battle_sim::shot_fired::ShotFired) PER ROUND
//! in one frame.
//! [`spawn_shot_projectiles`](spawn::spawn_shot_projectiles) drains them in read order and
//! gives each round a
//! staggered LAUNCH DELAY (its read-order index × the hot-reloadable
//! [`InterShotSeconds`](super::tuning::InterShotSeconds)) so the volley animates
//! SHOT-BY-SHOT rather than all bolts leaving the muzzle at once: a
//! projectile is held INVISIBLE at the muzzle until its launch delay elapses, then
//! runs its velocity travel. So a burst reads as distinct sequential rounds (GTW-308).
//!
//! Pure VIEW (ADR-0001): it only READS [`ShotFired`](gdtf_battle_sim::shot_fired::ShotFired) + the
//! rendered ganger
//! [`Transform`](bevy::prelude::Transform)s + draws sprites; it adds no sim plumbing and
//! never writes the sim.
//! Param-only throughout (`bevy-traps.md` #7).

mod advance;
mod pending;
mod spawn;
mod travel;

pub use advance::advance_projectiles;
pub use pending::PendingImpact;
pub(in crate::actors::fx) use spawn::spawn_pops_at_anchor;
pub use spawn::spawn_shot_projectiles;
pub use travel::{ProjectileTravel, ShotProjectile};
