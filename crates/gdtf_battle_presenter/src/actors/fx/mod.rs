//! Transient FX flashes (GTW-48 S6 / GTW-220): the presenter's one-shot FX layer.
//!
//! This module turns the three already-landed sim FX MESSAGES into short-lived 16x16
//! FX sprites drawn from the effects sheet ([`SheetRole::Effects`](crate::SheetRole),
//! `assets/sprites/alt_tileset_effects.png`):
//!
//! - [`Bleeding`](gdtf_battle_sim::Bleeding) `{ ganger }` -> a blood/hit FLASH sprite at
//!   the ganger's cell ([`read_bleeding`]); the flash's intensity is a RELATION to the
//!   ganger's [`Wounds`](gdtf_battle_sim::Wounds) (the message carries NO amount — verified
//!   `bleed.rs:62-65`), read from a `Query<&Wounds>`, never a pinned literal.
//! - [`ArmorBroken`](gdtf_battle_sim::ArmorBroken) `{ ganger, part }` -> a spark/break flash
//!   at the ganger's cell ([`read_armor_broken`]).
//! - [`CoverDestroyed`](gdtf_battle_sim::CoverDestroyed) `{ at }` -> a debris/rubble burst at
//!   `cell_to_world(at)` ([`read_cover_destroyed`]); ADDITIVE to the S4 rubble swap.
//! - [`FallOccurred`](gdtf_battle_sim::FallOccurred) `{ ganger, to_level, storeys, … }` ->
//!   a fall-impact flash at the landing cell + a `"Fell"` FCT pop (GTW-524;
//!   [`read_fall_occurred`]); ADDITIVE to the wound/bleed/injury flashes the fall damage drives.
//! - [`ShotFired`](gdtf_battle_sim::ShotFired) `{ muzzle, trajectory, impact, kind, damage }`
//!   -> the GTW-306 FIRING FX (reshaped from GTW-290's smeared stretched tracer): a
//!   **traveling directional projectile** that LERPS muzzle→impact then despawns
//!   ([`spawn_shot_projectiles`] / [`advance_projectiles`]), and a **3-frame impact
//!   animation** at its arrival point ([`animate_impact`], FX-B). The projectile + impact
//!   tiles are PER DAMAGE TYPE (the [`ShotFired`](gdtf_battle_sim::ShotFired) `damage` selects the color row; the
//!   trajectory selects the 8-way direction). The standalone muzzle flash was REMOVED
//!   (GTW-307): it rendered oversized at the shooter's feet and read poorly, so the
//!   traveling projectile (departing the muzzle) IS the fire signal. It is the generic firing
//!   FX ONLY; it does NOT duplicate the three CONSEQUENCE flashes above (a hit on a ganger
//!   still bleeds via `Bleeding`, breaks armor via `ArmorBroken`, etc.). One projectile per
//!   ROUND — a burst / full-auto shot spawns one per round, STAGGERED so the volley animates
//!   shot-by-shot.
//!
//! Every spawned flash carries a [`FlashTtl`] lifetime + an [`FxFlash`] marker; the
//! [`expire_flashes`] system ticks each [`FlashTtl`] with `Res<Time>` and despawns the
//! flash on expiry — THAT expiry is what makes the flashes one-shot / transient. Multiple
//! messages for the same ganger/cell in one frame each spawn an INDEPENDENT short-lived
//! sprite (NO coalescing this slice).
//!
//! WHICH effect tile each FX draws is DATA-DRIVEN: a per-line-commented
//! `assets/sprites/effect_roles.spritedef.ron`, loaded through the SAME generic
//! [`RonAsset<T>`](gdtf_assets::RonAsset) loader S4's `tile_roles.ron` uses, mapping each FX
//! to a [`TileIndex`](crate::TileIndex). Nothing about the index choices is hardcoded in
//! Rust.
//!
//! It only READS the three sim messages (+ a `Query<&Position>` / `Query<&Wounds>`) and adds
//! ZERO sim setup/teardown. It mirrors, never owns, combat truth — the one-way
//! `input -> presenter -> sim` edge (ADR-0001); the sim never reads the presenter.

mod fall;
mod fct;
mod flash;
mod impact;
mod melee;
mod projectile;
mod readers;
mod roles;
mod tuning;

#[cfg(test)]
mod test;

pub use fall::read_fall_occurred;
pub use fct::{
    CombatLogEvent, CombatText, FctEmphasis, FctStackIndex, FctValence, FloatingCombatText,
    InjuryLogText, LogLine, LogName, animate_floating_text, classify_log_event,
    read_consequence_fct, read_injury_fct, read_suppression_fct, severity_color,
    spawn_floating_text, valence_color,
};
pub use flash::{FlashTtl, FxFlash, expire_flashes};
pub use impact::{ShotImpactResolved, animate_impact};
pub use melee::read_melee_resolved;
pub use projectile::{
    PendingImpact, ProjectileTravel, ShotProjectile, advance_projectiles, spawn_shot_projectiles,
};
pub use readers::{read_armor_broken, read_bleeding, read_cover_destroyed};
pub use roles::{
    COMPASS_DIRECTIONS, DIRECTION_COUNT, DamageTypeFx, EffectRoles, EffectRolesHandle,
    IMPACT_FRAME_COUNT, load_effect_roles, nearest_direction_index,
    redrive_effect_roles_on_asset_event, resolve_effect_roles,
};
pub use tuning::{
    FctRiseRate, FctTtlSeconds, FxTuning, FxTuningHandle, ImpactFrameSeconds, InterShotSeconds,
    ProjectileDrawScale, ProjectileVelocity, load_fx_tuning, redrive_fx_tuning_on_asset_event,
    resolve_fx_tuning,
};
