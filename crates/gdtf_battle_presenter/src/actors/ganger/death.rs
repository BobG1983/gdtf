//! Sprite lifecycle-out: the death despawn (with the GTW-331 shot-kill deferral) and
//! the removal despawn.

use bevy::{ecs::lifecycle::RemovedComponents, prelude::*};
use gdtf_battle_sim::{
    prelude::{LifeState, Position},
    resolve_and_apply::HitReport,
    shot_fired::ShotFired,
};

use super::sprite_map::GangerSprites;
use crate::{
    ShotImpactResolved,
    playback::{DrawnLife, Played},
};

/// `Update` (`PresenterSystems::Scene`): despawn the sprite of a ganger whose
/// [`Changed<LifeState>`] reached [`Dead`](LifeState::Dead) — discriminating a shot-kill
/// (deferred to the impact despawn) from a non-shot death (despawned promptly).
///
/// A [`Dead`](LifeState::Dead) ganger's presenter sprite is DESPAWNED and its
/// [`GangerSprites`] entry dropped (the contract's chosen death-delta — despawn, not a
/// corpse tile). This is the system's ONLY job (GTW-631 C3): the Downed grey-out /
/// revive re-tint a life-state change also implies is the appearance resolver's
/// ([`resolve_ganger_appearance`](super::appearance::resolve_ganger_appearance), which
/// keys on the same `Changed<LifeState>`), so this system writes no [`Sprite`] field.
///
/// GTW-331 — the death-despawn DISCRIMINATOR. The sim flips a shot-killed ganger's
/// [`LifeState`] to [`Dead`](LifeState::Dead) AND emits the killing
/// [`ShotFired`](gdtf_battle_sim::shot_fired::ShotFired) in the SAME tick (`dispatch_fire` — `fire()`
/// applies the damage then writes the per-round signal), so the presenter observes BOTH on the
/// same drain update — well BEFORE the staggered killing tracer flies + impacts (GTW-308). Were
/// the sprite despawned here, the body would vanish at sim-drain time, before its tracer lands.
///
/// So a [`Dead`](LifeState::Dead) ganger is despawned here ONLY when NO killing shot for it
/// arrived this frame — i.e. no drained [`ShotFired`] whose threaded
/// [`HitReport`](gdtf_battle_sim::resolve_and_apply::HitReport) names this ganger (a
/// [`HitVerdict::Ganger`](gdtf_battle_sim::resolve_and_apply::HitVerdict::Ganger) verdict) and left it
/// [`Dead`](LifeState::Dead). That is a NON-shot death (a bleed-out, a directly-set state — no
/// tracer is coming), so it despawns promptly. When a killing shot DID arrive, the despawn is
/// DEFERRED to [`despawn_killed_ganger_on_impact`], which drains the bolt's
/// [`ShotImpactResolved`](crate::ShotImpactResolved) signal at the staggered impact moment — so the
/// body lives until the tracer reaches it, then despawns exactly once.
///
/// `ShotFired` is the reliable discriminator (NOT a query of the in-flight bolt): the bolt is
/// spawned via a deferred `commands.spawn_scene` and its
/// [`ProjectileTravel`](crate::fx::ProjectileTravel) materializes only on
/// a later schedule, so it is NOT queryable on the drain frame the life-state flips — but the
/// `ShotFired` message IS present that exact frame (its own reader cursor, independent of the
/// projectile spawner's). The [`GangerSprites`] entry is dropped on whichever path despawns the
/// sprite, never both (a shot-kill is guarded out here and despawned at impact; a non-shot death
/// has no impact to fire).
///
/// Param-only (`bevy-traps.md` #7): [`Commands`], [`ResMut<GangerSprites>`], the
/// changed-life query, and the
/// [`MessageReader<ShotFired>`](gdtf_battle_sim::shot_fired::ShotFired) the discriminator drains for this
/// frame's killing shots.
pub fn update_ganger_life_state(
    mut commands: Commands,
    mut sprites: ResMut<GangerSprites>,
    changed: Query<(Entity, &DrawnLife), Changed<DrawnLife>>,
    mut shots: MessageReader<Played<ShotFired>>,
) {
    // The set of gangers a killing shot arrived for THIS frame — those deaths are pending an
    // incoming tracer, so the despawn is deferred to `despawn_killed_ganger_on_impact`. Built
    // once per run from this frame's `ShotFired` (its own reader cursor; the projectile spawner
    // drains the buffer through a separate cursor).
    let shot_killed: Vec<Entity> = shots
        .read()
        .filter_map(|played| shot_kill_victim(played))
        .collect();
    for (entity, drawn) in &changed {
        let life = &**drawn;
        // Only a DEATH is lifecycle-out; the Downed / revive re-tint rides the appearance
        // resolver (GTW-631), which keys on this same `Changed<LifeState>`.
        if !matches!(life, LifeState::Dead) {
            continue;
        }
        // A shot-kill whose tracer has not yet landed (a killing `ShotFired` arrived this
        // frame naming this ganger) is left alive on screen —
        // `despawn_killed_ganger_on_impact` despawns it when the killing shot's
        // `ShotImpactResolved` fires. Only a NON-shot death (no killing shot this frame)
        // despawns promptly here.
        if shot_killed.contains(&entity) {
            continue;
        }
        // Despawn the presenter sprite and drop its map entry (a non-shot death).
        if let Some(presenter) = sprites.remove(entity) {
            commands.entity(presenter).despawn();
        }
    }
}

/// The sim ganger [`Entity`] a [`ShotFired`](gdtf_battle_sim::shot_fired::ShotFired) KILLED, if it struck a
/// ganger (a [`HitVerdict::Ganger`](gdtf_battle_sim::resolve_and_apply::HitVerdict::Ganger) verdict) AND left it
/// [`Dead`](LifeState::Dead) (GTW-331) — else [`None`]
/// (a non-ganger hit, a non-lethal hit, a clean miss, or a geometry-only round).
///
/// [`update_ganger_life_state`]'s discriminator reads it over this frame's `ShotFired` to find
/// deaths pending a tracer; it delegates to [`report_kill_victim`] so the fire-frame guard and the
/// impact-frame despawn ([`despawn_killed_ganger_on_impact`]) classify a kill IDENTICALLY.
fn shot_kill_victim(shot: &ShotFired) -> Option<Entity> {
    report_kill_victim(shot.report.as_ref())
}

/// The sim ganger [`Entity`] a shot's [`HitReport`](gdtf_battle_sim::resolve_and_apply::HitReport) KILLED, if it
/// struck a ganger (a [`HitVerdict::Ganger`](gdtf_battle_sim::resolve_and_apply::HitVerdict::Ganger) verdict) AND
/// left it [`Dead`](LifeState::Dead) (GTW-331) — else
/// [`None`].
///
/// The shared kill-classifier both death-despawn halves use over the SAME sim verdict: the
/// fire-frame guard reads it off the [`ShotFired`](gdtf_battle_sim::shot_fired::ShotFired)'s threaded report
/// (via [`shot_kill_victim`]), the impact-frame despawn reads it off the
/// [`ShotImpactResolved`](crate::ShotImpactResolved)'s threaded report — so a kill is the same
/// kill on both ends (the deferral and the despawn never disagree). The [`Entity`] is framework
/// plumbing (the no-bare-types carve-out).
fn report_kill_victim(report: Option<&HitReport>) -> Option<Entity> {
    let report = report?;
    // The GTW-573 per-kind verdict: only a landed ganger verdict can carry a kill (a
    // corpse-skip / miss folds to no-effect and names no victim).
    let gdtf_battle_sim::resolve_and_apply::HitVerdict::Ganger(verdict) = &report.verdict else {
        return None;
    };
    (verdict.applied.life_after == LifeState::Dead).then_some(verdict.target)
}

/// `Update` (`PresenterSystems::Scene`): despawn the presenter sprite of a ganger killed by a
/// shot WHEN the killing tracer lands (GTW-331).
///
/// Drains [`MessageReader<ShotImpactResolved>`](crate::ShotImpactResolved) — the shared per-shot
/// impact-resolved signal [`animate_impact`](crate::animate_impact) emits at each staggered impact
/// (GTW-328). For every signal whose threaded
/// [`HitReport`](gdtf_battle_sim::resolve_and_apply::HitReport) struck a ganger (a
/// [`HitVerdict::Ganger`](gdtf_battle_sim::resolve_and_apply::HitVerdict::Ganger) verdict) AND left it
/// [`Dead`](LifeState::Dead) ([`life_after`](gdtf_battle_sim::resolve_and_apply::AppliedDamage::life_after)), it
/// despawns that ganger's mapped presenter sprite and drops its [`GangerSprites`] entry — so the
/// body vanishes the instant its killing bolt arrives, not at sim-drain time.
///
/// This is the SHOT-KILL half of the death-despawn (the NON-shot half stays in
/// [`update_ganger_life_state`], which despawns only a death with no pending tracer). The two are
/// disjoint: a shot-kill is guarded OUT of the life-state path (a tracer is pending) and despawned
/// here; a non-shot death has no impact signal and is despawned there — so the [`GangerSprites`]
/// entry is dropped EXACTLY ONCE. A signal for an already-despawned / unmapped ganger (e.g. a
/// non-lethal hit, or a corpse the life-state path already removed) is a no-op
/// (`GangerSprites::remove` returns [`None`]).
///
/// Param-only (`bevy-traps.md` #7): [`Commands`], [`ResMut<GangerSprites>`], and the
/// [`MessageReader<ShotImpactResolved>`](crate::ShotImpactResolved).
pub fn despawn_killed_ganger_on_impact(
    mut commands: Commands,
    mut sprites: ResMut<GangerSprites>,
    mut impacts: MessageReader<ShotImpactResolved>,
) {
    for impact in impacts.read() {
        // Classify the resolved impact with the SAME kill-classifier the fire-frame guard uses
        // (`report_kill_victim`): a non-ganger / non-lethal / miss / geometry-only round yields
        // no victim and is skipped.
        let Some(victim) = report_kill_victim(impact.report.as_ref()) else {
            continue;
        };
        // The killing tracer has landed: despawn the struck ganger's sprite + drop its map entry
        // (a no-op if the life-state path already removed it / it was never mapped).
        if let Some(presenter) = sprites.remove(victim) {
            commands.entity(presenter).despawn();
        }
    }
}

/// `Update` (`PresenterSystems::Scene`): despawn the presenter sprite of a ganger whose
/// [`Position`] was REMOVED.
///
/// Drains [`RemovedComponents<Position>`] (from `bevy::ecs::removal_detection`); for each
/// removed sim entity it despawns the mapped presenter sprite and drops its
/// [`GangerSprites`] entry. A ganger losing its [`Position`] (e.g. removed from the
/// battle) leaves no orphan sprite behind.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`], [`ResMut<GangerSprites>`],
/// [`RemovedComponents<Position>`].
pub fn despawn_removed_ganger_sprites(
    mut commands: Commands,
    mut sprites: ResMut<GangerSprites>,
    mut removed: RemovedComponents<Position>,
) {
    for entity in removed.read() {
        if let Some(presenter) = sprites.remove(entity) {
            commands.entity(presenter).despawn();
        }
    }
}
