//! The GTW-507 close-combat **strike** FX — a one-frame strike glyph at the struck target
//! cell when a melee blow lands.
//!
//! [`read_melee_resolved`] drains the sim's [`MeleeResolved`](gdtf_battle_sim::acts::MeleeResolved)
//! signal (emitted once per CONNECTING melee hit by the sim's `dispatch_melee`) and spawns ONE
//! short-lived strike sprite at `cell_to_world(at)`, drawn from the data-driven
//! [`EffectRoles::melee_strike`](super::roles::EffectRoles) tile and tinted per the strike's
//! [`DamageType`](gdtf_battle_sim::DamageType). It MIRRORS the
//! [`read_cover_destroyed`](super::readers::read_cover_destroyed) pattern (the cell-keyed FX
//! flash) and reuses the SAME [`spawn_flash`](super::readers::spawn_flash) one-shot recipe
//! ([`FlashTtl`](super::flash::FlashTtl) + [`FxFlash`](super::flash::FxFlash)), so the glyph is
//! transient — the `expire_flashes` clock despawns it after its short TTL, i.e. the one-frame
//! strike read.
//!
//! Pure VIEW (ADR-0001): it READS the sim's [`MeleeResolved`] signal + the data-driven
//! [`EffectRoles`](super::roles::EffectRoles) table and draws a sprite; it NEVER writes the sim
//! (the one-way `sim → presenter` dep — the sim never reads the presenter). Param-only
//! throughout (`bevy-traps.md` #7).

use bevy::prelude::*;
use gdtf_battle_sim::{Cell, DamageType, Level, acts::MeleeResolved};

use super::{readers::spawn_flash, roles::EffectRoles};
use crate::{TopDownAtlases, cell_to_world, fx::readers::fx_sprite};

/// The strike-glyph tint for a melee hit's [`DamageType`] — a per-type colour hint so the
/// strike reads as the wielded weapon's flavour (a blunt Kinetic thud vs a Rend power-edge
/// flash), the melee mirror of the [`ShotFired`](gdtf_battle_sim::ShotFired) projectile's
/// per-damage-type colour rows.
///
/// A presenter-LOCAL colour choice (like `bleed_tint`) keyed off the
/// [`MeleeResolved`](gdtf_battle_sim::acts::MeleeResolved) `damage` the sim carried for exactly
/// this purpose — never a sim value, never a pinned magnitude a test asserts. The tile itself is
/// the DATA-DRIVEN `melee_strike` index of [`EffectRoles`](super::roles::EffectRoles); this tint
/// layers the weapon flavour on top of it. Total over the seven damage types (an exhaustive
/// match, no fallback path).
const fn strike_tint(damage: DamageType) -> Color {
    match damage {
        // Warm impact reads (blunt / explosive) — an orange-white flash.
        DamageType::Kinetic | DamageType::Blast => Color::srgb(1.0, 0.75, 0.35),
        // Beam / arc reads — a cyan-white flash.
        DamageType::Las | DamageType::Shock => Color::srgb(0.55, 0.85, 1.0),
        // Toxin / acid read — a sickly green flash.
        DamageType::Chem => Color::srgb(0.55, 1.0, 0.45),
        // Superheated / power-edge reads — a violet flash.
        DamageType::Plasma | DamageType::Rend => Color::srgb(0.85, 0.5, 1.0),
    }
}

/// `Update` (`PresenterSystems::Draw`): spawn a one-frame STRIKE glyph per
/// [`MeleeResolved`](gdtf_battle_sim::acts::MeleeResolved) (GTW-507).
///
/// Drains [`MessageReader<MeleeResolved>`](gdtf_battle_sim::acts::MeleeResolved); for each
/// `MeleeResolved { at, damage }` it reconstructs the typed [`Cell`] / [`Level`] from `at`
/// ([`CellLevel`](gdtf_battle_sim::CellLevel) Derefs to `IVec3`) and spawns ONE FX flash at
/// [`cell_to_world`](crate::cell_to_world)`(cell, level)` carrying
/// [`FlashTtl`](super::flash::FlashTtl) + [`FxFlash`](super::flash::FxFlash), with the table's
/// data-driven `melee_strike` [`TileIndex`](crate::TileIndex) (never a literal) tinted per the
/// strike's [`DamageType`] (the `strike_tint` per-type colour hint). The transient flash is the
/// strike's ONLY presenter job; the HP/wound consequences ride the existing change-detection +
/// wound/injury signals.
///
/// A missing effects sheet skips the flash FAIL-CLOSED (the shared `fx_sprite` recipe returns
/// [`None`], the loop `continue`s — no panic). The sim emits `MeleeResolved` ONLY on a connecting hit, so a missed
/// strike draws NO glyph (the §7 connect gate). It mirrors
/// [`read_cover_destroyed`](super::readers::read_cover_destroyed) exactly (the cell-keyed FX
/// flash) — it reads only the sim signal + the data table and never writes the sim (the one-way
/// `sim → presenter` dep).
///
/// Param-only (`bevy-traps.md` #7): [`Commands`], [`Res<TopDownAtlases>`],
/// [`Res<EffectRoles>`], and [`MessageReader<MeleeResolved>`].
pub fn read_melee_resolved(
    mut commands: Commands,
    atlases: Res<TopDownAtlases>,
    roles: Res<EffectRoles>,
    mut resolved: MessageReader<MeleeResolved>,
) {
    for msg in resolved.read() {
        // MeleeResolved.at is a CellLevel; reconstruct its typed Cell / Level (the z is a
        // storey index, clamped panic-free if impossibly out of range — the read_cover_destroyed
        // idiom).
        let cell = Cell::new(msg.at.x, msg.at.y);
        let storey = u8::try_from(msg.at.z).unwrap_or(0);
        let level = Level::new(storey);
        let Some(sprite) = fx_sprite(roles.melee_strike, strike_tint(msg.damage), &atlases) else {
            // Fail-closed: no effects sheet -> no glyph, no panic.
            continue;
        };
        spawn_flash(&mut commands, sprite, cell_to_world(cell, level));
    }
}
