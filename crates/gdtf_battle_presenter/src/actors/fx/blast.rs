//! The GTW-546 grenade-BLAST FX — the `AoE` explosion animation drawn at a lobbed grenade's
//! landing cell when a throw resolves.
//!
//! A thrown grenade's damage is applied entirely inside the sim
//! ([`resolve_blast`](gdtf_battle_sim::shot_pipeline::fire::resolve_blast) folds each covered
//! ganger through the SAME GTW-541 wound path) — but that fold emits NO
//! [`ShotFired`](gdtf_battle_sim::shot_fired::ShotFired), so the firing FX pipeline
//! ([`spawn_shot_projectiles`](super::projectile::spawn_shot_projectiles) →
//! [`animate_impact`](super::impact::animate_impact)) never draws anything for it. Like the
//! GTW-547 on-death explode, the blast would otherwise be an INVISIBLE HP drain (the struck
//! gangers' wounds tick down via change-detection + the bleed / injury signals, but nothing
//! reads at the point of detonation). The dedicated [`ThrowResolved`](gdtf_battle_sim::acts::ThrowResolved)
//! signal the sim emits per resolved throw is the presenter's hook: it carries the arc's LANDING
//! cell + the grenade's [`DamageType`](gdtf_battle_sim::weapon::DamageType).
//!
//! [`read_throw_resolved`] drains that signal and, at the landing cell, SEEDS a
//! [`PendingImpact`](super::projectile::PendingImpact) — the SAME seam a straight shot's arrived
//! bolt hands off — so the EXISTING [`animate_impact`](super::impact::animate_impact) plays the
//! grenade's damage-type 3-frame impact/shockwave strip
//! ([`EffectRoles::fx_for`](super::roles::EffectRoles::fx_for)`(damage).impact`, the expanding
//! burst → ring the GTW-541 `AoE` hits already render) at the detonation point. It builds NO new
//! blast infrastructure: the blast reads as the same expanding impact the fire path draws, keyed
//! off the sim's landing cell. The struck gangers' HP/wound consequences ride the existing
//! change-detection + wound/bleed/injury signals (this reader is ONLY the detonation glyph).
//!
//! No projectile flies for a lob this slice (the arc geometry is deterministic in the sim; a
//! traveling arc sprite is optional polish, not the requirement — the blast + the button are).
//! The seeded impact carries NO floating-combat-text pops (the blast's numbers ride the wound /
//! injury FCT signals the fold already drives) and a [`None`] report + a
//! [`PLACEHOLDER`](bevy::ecs::entity::Entity::PLACEHOLDER) shooter (a blast has no single
//! `ShotImpactResolved` verdict — it fans over many gangers; the combat log reacts to those
//! per-ganger signals, not this detonation moment) — the GTW-547 on-death-marker precedent for
//! a signal-keyed FX with no per-shot verdict.
//!
//! Pure VIEW (ADR-0001): it READS the sim's [`ThrowResolved`](gdtf_battle_sim::acts::ThrowResolved)
//! signal + the data-driven [`EffectRoles`](super::roles::EffectRoles) impact strip (via the
//! shared impact seam) and draws a sprite; it NEVER writes the sim (the one-way `sim → presenter`
//! dep — the sim never reads the presenter). Param-only throughout (`bevy-traps.md` #7).

use bevy::{
    ecs::{message::MessageReader, template::template},
    prelude::*,
    scene::{CommandsSceneExt, bsn},
};
use gdtf_battle_sim::acts::ThrowResolved;

use super::projectile::PendingImpact;
use crate::cell_to_world;

/// `Update` (`PresenterSystems::Overlay`): draw the grenade BLAST explosion per
/// [`ThrowResolved`](gdtf_battle_sim::acts::ThrowResolved) (GTW-546).
///
/// Drains [`MessageReader<ThrowResolved>`](gdtf_battle_sim::acts::ThrowResolved); for each
/// `ThrowResolved { at, damage }` it reconstructs the typed [`Cell`](gdtf_battle_sim::metric::Cell) /
/// [`Level`](gdtf_battle_sim::metric::Level) from the landing
/// [`CellLevel`](gdtf_battle_sim::metric::CellLevel) (the `read_melee_resolved` idiom — `at` Derefs to
/// `IVec3`; the storey clamps panic-free if impossibly out of range) and SEEDS a
/// [`PendingImpact`](super::projectile::PendingImpact) at
/// [`cell_to_world`](crate::cell_to_world)`(cell, level)` carrying the grenade's
/// [`DamageType`](gdtf_battle_sim::weapon::DamageType). The EXISTING
/// [`animate_impact`](super::impact::animate_impact) (registered in the same FX band) picks the
/// seed up next update and plays that damage type's 3-frame expanding-shockwave impact strip at
/// the detonation point — the SAME `AoE` hit FX the fire path renders, reused verbatim (no new
/// blast infrastructure).
///
/// The seed carries an EMPTY pop list, a [`None`] report, and a
/// [`PLACEHOLDER`](bevy::ecs::entity::Entity::PLACEHOLDER) shooter: a blast is a multi-ganger fan
/// with no single per-shot verdict, so its numbers + downs ride the per-ganger wound / injury /
/// bleed FCT signals the sim's blast fold already drives — this reader is ONLY the detonation
/// glyph (the GTW-547 on-death-marker precedent).
/// [`animate_impact`](super::impact::animate_impact) still emits its
/// [`ShotImpactResolved`](super::impact::ShotImpactResolved) with the placeholder shooter + `None`
/// report; the combat log renders NO outcome line for a verdict-less `None` report (GTW-559), so
/// the blast adds no phantom shot outcome.
///
/// A [`PendingImpact`] is seeded whether or not the effects sheet is loaded (it is spawned
/// unconditionally); [`animate_impact`](super::impact::animate_impact) itself fails the impact
/// SPRITE closed when the sheet is
/// absent (its `fx_sprite_scaled` returns [`None`]) — so a missing sheet degrades to no glyph, no
/// panic, matching every other FX reader. It mirrors
/// [`read_melee_resolved`](super::melee::read_melee_resolved) (a cell-keyed FX off a `*Resolved`
/// signal), reading only the sim signal + drawing, never writing the sim.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] and
/// [`MessageReader<ThrowResolved>`](gdtf_battle_sim::acts::ThrowResolved).
pub fn read_throw_resolved(mut commands: Commands, mut resolved: MessageReader<ThrowResolved>) {
    for msg in resolved.read() {
        // ThrowResolved.at is a CellLevel; its typed Cell / Level via the canonical
        // CellLevel::split decompose (GTW-565).
        let (cell, level) = msg.at.split();
        let at = cell_to_world(cell, level);
        // Seed the SHARED impact seam at the landing so `animate_impact` plays the grenade's
        // damage-type 3-frame expanding-shockwave strip there — the existing AoE hit FX, reused.
        // No pops / no verdict: the blast's numbers ride the per-ganger wound/injury FCT signals.
        let pending = PendingImpact::for_blast(at, msg.damage);
        commands.spawn_scene(bsn! { template(move |_| Ok(pending.clone())) });
    }
}
