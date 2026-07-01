//! The change-driven ganger-sprite spawn / move / reframe / life / removal / level-filter
//! systems and their per-ganger projection helpers.

use bevy::{
    camera::visibility::RenderLayers,
    ecs::{lifecycle::RemovedComponents, message::MessageReader, template::template},
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
};
use gdtf_battle_sim::{
    Aiming, Cell, Facing, Faction, HitReport, Level, LifeState, Position, ShotFired, ShotKind,
    Stance, Suppressed,
};

use super::{
    frame::atlas_index,
    roles::CharacterRoles,
    sprite_map::{GangerSprite, GangerSprites},
    tint::{ganger_tint, stance_aiming_tint},
    tween::SpriteTween,
};
use crate::{
    ActiveLevel, CELL_PX, Layer, SheetRole, ShotImpactResolved, TopDownAtlases, ViewMode,
    cell_to_world_layered,
};

/// The world `(cell, level)` a ganger's [`Position`] projects to — reconstruct the typed
/// [`Cell`] / [`Level`] from the position's `IVec3` components (the S4 idiom), since
/// [`Position`] Derefs to [`CellLevel`](gdtf_battle_sim::CellLevel) Derefs to `IVec3`.
///
/// `pos.z` is a storey index in `0..MAX_LEVELS`; clamping the (impossible-in-practice)
/// negative / over-`u8` case keeps the reconstruction panic-free.
fn cell_and_level(pos: &Position) -> (Cell, Level) {
    let cell = Cell::new(pos.x, pos.y);
    let storey = u8::try_from(pos.z).unwrap_or(0);
    (cell, Level::new(storey))
}

/// Whether a ganger at `pos` is DRAWN — i.e. its storey lies within the drawn band under the
/// current [`ViewMode`] (GTW-520 C4, widened for the GTW-521 view toggle).
///
/// Consults the ONE shared band predicate
/// [`ActiveLevel::draws_storey`](crate::ActiveLevel::draws_storey), the successor to the
/// pre-GTW-520 on-active-storey hard cut: a ganger on ANY storey within the drawn band is
/// drawn (it peeks through floor-gaps on the lower storeys GTW-519 already renders terrain
/// for), and one strictly ABOVE the band ceiling is culled. Every ganger-visibility
/// site ([`spawn_ganger_sprites`] / [`move_ganger_sprites`] / [`apply_active_level_filter`]
/// AND the fog writer's `present_actor_fog`) routes through this SAME predicate so they
/// cannot drift.
///
/// The [`ViewMode`] chooses the band CEILING (GTW-521 C2): [`ViewMode::DownToActive`] caps at
/// the active level (unchanged GTW-520); [`ViewMode::FullView`] draws every storey. It
/// reconstructs the typed [`Level`] from the position's `z` (the S4 idiom — clamping the
/// impossible negative / over-`u8` case keeps it panic-free) and asks the [`ActiveLevel`]
/// whether that storey is drawn under `view`. The move / filter systems have the
/// [`ActiveLevel`] + [`ViewMode`] as [`Res`]; the actor fog arm holds the dereferenced values
/// and rebuilds one via [`ActiveLevel::new`] — both reach the same predicate.
fn ganger_in_drawn_band(pos: &Position, active: ActiveLevel, view: ViewMode) -> bool {
    let (_cell, level) = cell_and_level(pos);
    active.draws_storey(level, view)
}

/// Build one ganger [`Sprite`] on the character sheet at `index`, tinted `tint`, via the
/// S3 recipe.
///
/// `Sprite::from_atlas_image(chars.image, TextureAtlas { layout, index })` with
/// `custom_size = Some(Vec2::splat(CELL_PX))` (the documented S3 sizing recipe) and the
/// `tint` applied to `Sprite.color`. Returns [`None`] if the character sheet was not
/// loaded (so the caller skips the spawn rather than panic).
fn ganger_sprite(index: usize, tint: Color, atlases: &TopDownAtlases) -> Option<Sprite> {
    let chars = atlases.role(SheetRole::Characters)?;
    let mut sprite = Sprite::from_atlas_image(
        chars.image.clone(),
        TextureAtlas {
            layout: chars.layout.clone(),
            index,
        },
    );
    sprite.custom_size = Some(Vec2::splat(CELL_PX));
    sprite.color = tint;
    Some(sprite)
}

/// `Update` (`PresenterSystems::Draw`): spawn one presenter sprite per newly-added
/// ganger within the drawn storey band.
///
/// For every ganger whose [`Position`] was [`Added`] this update, build a [`Sprite`] (atlas
/// index `faction_base + facing_frame`, the faction tint) at the [`Layer::Actor`](crate::Layer)
/// projection ([`cell_to_world_layered`](crate::cell_to_world_layered) — the cell's world
/// position at the ganger's OWN [`Level`], lifted by [`GANGER_Z_BIAS`](crate::GANGER_Z_BIAS)
/// so it draws over its own floor tile, GTW-283), on the
/// [`WORLD_RENDER_LAYER`](crate::WORLD_RENDER_LAYER), with the [`GangerSprite`] marker; record
/// `sim Entity -> presenter Entity` in [`GangerSprites`].
///
/// GTW-520 (C1/C2): the sprite is spawned SHOWN when the ganger's storey lies within the drawn
/// band `0..=active` ([`ganger_in_drawn_band`], the shared
/// [`ActiveLevel::draws_storey`](crate::ActiveLevel::draws_storey) predicate) — so a ganger on
/// a LOWER storey is drawn at its own storey's Z and peeks through floor-gaps — and spawned
/// HIDDEN when it is strictly ABOVE the active level. An off-band ganger's
/// [`Visibility::Hidden`] sprite is recorded too so a later [`apply_active_level_filter`] can
/// show it without a respawn — the band filter is uniform across spawn / move / level-change /
/// the fog writer.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`], [`ResMut<GangerSprites>`], the read
/// resources, and the [`Added<Position>`] ganger query.
pub fn spawn_ganger_sprites(
    mut commands: Commands,
    mut sprites: ResMut<GangerSprites>,
    roles: Res<CharacterRoles>,
    atlases: Res<TopDownAtlases>,
    active: Res<ActiveLevel>,
    view: Res<ViewMode>,
    added: Query<(Entity, &Position, &Faction, &Facing, &LifeState), Added<Position>>,
) {
    for (entity, pos, faction, facing, life) in &added {
        // A Dead ganger added directly (no live frame) draws no sprite.
        if matches!(life, LifeState::Dead) {
            continue;
        }
        let index = atlas_index(&roles, *faction, *facing);
        let tint = ganger_tint(*faction, *life);
        let Some(sprite) = ganger_sprite(index, tint, &atlases) else {
            continue;
        };
        let (cell, level) = cell_and_level(pos);
        // GTW-520 C1/C2: a ganger anywhere in the DRAWN band (`0..=active`) spawns shown so it
        // peeks through floor-gaps on a lower storey; one strictly above the active level
        // spawns HIDDEN (later shown without a respawn by `apply_active_level_filter`). The
        // Actor-layer projection below uses the ganger's OWN `level`, so a lower-storey ganger
        // draws at its own storey's Z and occludes correctly.
        let visibility = if ganger_in_drawn_band(pos, *active, *view) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        // The Actor layer lifts the ganger by GANGER_Z_BIAS so it draws over its own
        // floor tile (GTW-283), without crossing into the next storey's band.
        let spawn_world = cell_to_world_layered(cell, level, Layer::Actor);
        let transform = Transform::from_translation(spawn_world);
        let layers = RenderLayers::layer(crate::WORLD_RENDER_LAYER);
        // GTW-359 (C4): seed a SETTLED movement tween at the spawn position so the move
        // path (`move_ganger_sprites`) always finds a tween to RE-TARGET rather than
        // special-casing the first move. A settled tween produces no motion until the
        // first `Changed<Position>` retargets it.
        let tween = SpriteTween::settled(spawn_world);
        // GTW-322 — authored as a `bsn!` scene. The atlas-indexed `Sprite` is NOT `Unpin`
        // (its `Option<Handle<Image>>` / `Option<TextureAtlas>` fields), so it rides
        // NEITHER `template_value` (which bounds `Unpin`) nor a `bsn!` field patch — it
        // takes the `template(move |_| Ok(value.clone()))` closure escape hatch (the
        // `FnTemplate` has no `Unpin` bound on its output), the same one the AREA-1 widget
        // builders use for `TextFont`. The runtime `Transform` / `Visibility` and
        // `RenderLayers` ARE `Clone + Default + Unpin`, so each rides `template_value` (a
        // value-overwrite). The `GangerSprite` marker carries a runtime sim `Entity` and
        // has no `Default` (so no `bsn!` / `template_value` form) — it is `.insert`ed onto
        // the synchronously-reserved id after the scene. `spawn_scene` reserves the id NOW
        // (so the `GangerSprites` map records a usable handle this update); the scene's
        // components materialize on the `SpawnScene` schedule (~one update later) — the
        // same entity + components result, only the spawn SHAPE changed. The move /
        // reframe / level-filter systems look the sprite up through the map and gracefully
        // skip until its components exist.
        let presenter = commands
            .spawn_scene((
                bsn! { template(move |_| Ok(sprite.clone())) },
                template_value(transform),
                template_value(visibility),
                template_value(layers),
                // GTW-359 (C4): the SpriteTween (a Timer + two Vec3s) has no Default, so —
                // like the ImpactAnimation — it rides the `template(move |_| Ok(value.clone()))`
                // closure escape hatch (the FnTemplate output is bound by neither Unpin nor
                // Default), not `template_value`.
                bsn! { template(move |_| Ok(tween.clone())) },
            ))
            .insert(GangerSprite { entity })
            .id();
        sprites.insert(entity, presenter);
    }
}

/// `Update` (`PresenterSystems::Draw`, `.after(spawn_ganger_sprites)`): RE-TARGET the
/// movement tween (do NOT respawn, do NOT snap) of a ganger whose [`Position`] changed,
/// and flip its [`Visibility`].
///
/// For every ganger whose [`Position`] is [`Changed`], look the presenter sprite up
/// through [`GangerSprites`] and:
///
/// - GTW-359 (C4): RE-TARGET its [`SpriteTween`] — source = the sprite's CURRENT (possibly
///   mid-glide) [`Transform`] translation, target = the new
///   [`Layer::Actor`](crate::Layer) projection of the cell
///   ([`cell_to_world_layered`](crate::cell_to_world_layered) — so the
///   [`GANGER_Z_BIAS`](crate::GANGER_Z_BIAS) lift holds across moves), restarting the
///   glide clock. The actual [`Transform`] write is the
///   [`advance_sprite_tweens`](super::advance_sprite_tweens) glide that runs
///   `.after` this; setting the source to the live translation means a sim that outruns
///   the tween keeps the sprite gliding continuously toward the latest cell — it NEVER
///   snaps and NEVER gates the sim (the sim's [`Position`] is authoritative; the tween
///   only smooths the view). Covers planar AND cross-storey moves (the GENERAL per-step
///   glide that subsumes GTW-361);
/// - GTW-520 (C4): flip its [`Visibility`] by whether the new `(cell, level)` is WITHIN the
///   drawn band `0..=active` ([`ganger_in_drawn_band`], the shared
///   [`ActiveLevel::draws_storey`](crate::ActiveLevel::draws_storey) predicate) — the
///   cross-storey handoff (Hidden only when the ganger moves strictly ABOVE the active level,
///   Inherited when it is at or below it, including onto a lower drawn storey where it peeks
///   through the floor-gaps). The tween restructured the [`Transform`] write (now via the
///   glide); this filter WIDENED from the old on-active-storey hard cut to band membership.
///
/// It does NOT spawn a second sprite: it is idempotent via the map (a just-`Added` ganger
/// handled by [`spawn_ganger_sprites`] this same update is already mapped —
/// `.after(spawn_ganger_sprites)` guarantees the entry + its seeded [`SpriteTween`] exist
/// — so this only re-targets the same tween; a not-yet-mapped ganger is skipped). The
/// contract's "idempotent via the map" move path.
///
/// Param-only (`bevy-traps.md` #7): [`Res<GangerSprites>`], [`Res<ActiveLevel>`], the
/// moved-ganger query, and the presenter-sprite [`Transform`] / [`SpriteTween`] /
/// [`Visibility`] query.
pub fn move_ganger_sprites(
    sprites: Res<GangerSprites>,
    active: Res<ActiveLevel>,
    view: Res<ViewMode>,
    moved: Query<(Entity, &Position), Changed<Position>>,
    mut presenters: Query<(&Transform, &mut SpriteTween, &mut Visibility), With<GangerSprite>>,
) {
    for (entity, pos) in &moved {
        let Some(presenter) = sprites.sprite_for(entity) else {
            continue;
        };
        let Ok((transform, mut tween, mut visibility)) = presenters.get_mut(presenter) else {
            continue;
        };
        let (cell, level) = cell_and_level(pos);
        // The Actor-layer lift (GANGER_Z_BIAS) must hold across moves too, so the moved
        // ganger keeps drawing over the floor tile at its new cell (GTW-283).
        let target = cell_to_world_layered(cell, level, Layer::Actor);
        // RE-TARGET from the sprite's CURRENT translation (possibly mid-glide) so the
        // glide is seamless across a rapid sequence of Changed<Position> (the sim never
        // gated, the sprite never snapped).
        tween.retarget(transform.translation, target);
        // GTW-520 C4: the cross-storey handoff now uses the shared DRAWN-BAND predicate
        // (`0..=active`), not the old on-active-storey hard cut — so a ganger that moves DOWN
        // onto a lower drawn storey stays shown (peeking through the floor-gaps) and only one
        // that moves strictly ABOVE the active level is hidden.
        *visibility = if ganger_in_drawn_band(pos, *active, *view) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

/// The read-only ganger fields the reframe/re-tint read — the [`QueryData`] tuple,
/// factored out to keep the [`reframe_ganger_sprites`] query under the
/// `type_complexity` clippy gate.
///
/// [`QueryData`]: bevy::ecs::query::QueryData
type ReframeData = (
    Entity,
    &'static Faction,
    &'static Facing,
    &'static Stance,
    &'static Aiming,
    &'static LifeState,
    Option<&'static Suppressed>,
);

/// The "any of facing / stance / aiming / suppressed changed" [`QueryFilter`] driving the
/// reframe/re-tint, factored out for the same `type_complexity` reason as [`ReframeData`].
///
/// [`Changed<Suppressed>`](Suppressed) catches suppression being APPLIED (the sim inserts the
/// component — GTW-526 C2); a suppressed ganger's auto-stance drop (C5) ALSO trips
/// [`Changed<Stance>`], so the tint is doubly guaranteed to refresh on application. Suppression
/// being CLEARED is a component REMOVAL (the sim `remove`s it — C6), which `Changed` does NOT
/// observe, so [`reframe_ganger_sprites`] additionally drains
/// [`RemovedComponents<Suppressed>`](RemovedComponents) to un-tint a no-longer-suppressed ganger.
///
/// [`QueryFilter`]: bevy::ecs::query::QueryFilter
type ReframeChanged = Or<(
    Changed<Facing>,
    Changed<Stance>,
    Changed<Aiming>,
    Changed<Suppressed>,
)>;

/// `Update` (`PresenterSystems::Draw`): reframe / re-tint a ganger sprite whose
/// [`Facing`], [`Stance`], [`Aiming`], or [`Suppressed`] state changed.
///
/// For every ganger whose [`Facing`] / [`Stance`] / [`Aiming`] / [`Suppressed`] is [`Changed`],
/// look the presenter sprite up through [`GangerSprites`] and recompute its texture-atlas index
/// (facing reframe via the 8->4 map) and re-tint it (the stance / aiming / suppressed delta) in
/// place. The reframe always recomputes from the CURRENT facing; the aiming delta
/// brightens the sprite (an aimed ganger reads "ready"); the stance delta dims a prone
/// ganger (a flattened silhouette); a [`Suppressed`] ganger is desaturated + darkened
/// (GTW-526 C8 — it reads distinctly "pinned"). A [`Dead`](LifeState::Dead) ganger's sprite is
/// already despawned, so its lookup misses and is skipped.
///
/// Suppression being CLEARED is a component REMOVAL, which `Changed` does not observe, so this
/// system ALSO drains [`RemovedComponents<Suppressed>`](RemovedComponents) and re-tints each
/// just-cleared ganger — its (now `Suppressed`-less) [`ReframeData`] is read back through the
/// full-set `all` query, so `stance_aiming_tint` sees `suppressed = false` and the ganger
/// returns to its ordinary faction tint.
///
/// Param-only (`bevy-traps.md` #7): [`Res<GangerSprites>`], the changed-state ganger query, the
/// full-set ganger query (for the removal re-read), the [`RemovedComponents<Suppressed>`], and
/// the presenter-sprite [`Sprite`] query.
pub fn reframe_ganger_sprites(
    sprites: Res<GangerSprites>,
    roles: Res<CharacterRoles>,
    changed: Query<ReframeData, ReframeChanged>,
    all: Query<ReframeData>,
    mut removed: RemovedComponents<Suppressed>,
    mut presenters: Query<&mut Sprite, With<GangerSprite>>,
) {
    // The APPLIED / stance / aim / facing path: every ganger whose reframe state Changed.
    for data in &changed {
        reframe_one(&sprites, &roles, &mut presenters, data);
    }
    // The CLEARED path: a suppression REMOVAL is not a `Changed`, so drain the removals and
    // re-tint each just-cleared ganger from its full-set data (now carrying no `Suppressed`,
    // so the tint drops back to the ordinary faction colour). A ganger whose sprite is already
    // gone (Dead) or whose entity despawned is skipped by the `all.get` / `sprite_for` misses.
    for entity in removed.read() {
        if let Ok(data) = all.get(entity) {
            reframe_one(&sprites, &roles, &mut presenters, data);
        }
    }
}

/// Reframe + re-tint one ganger's presenter sprite from its [`ReframeData`] — the shared body
/// of both [`reframe_ganger_sprites`] paths (the `Changed` loop and the suppression-removal
/// loop).
///
/// Looks the presenter sprite up through [`GangerSprites`], recomputes its facing-correct
/// atlas index, and re-tints it via [`stance_aiming_tint`] (the faction / life base modulated
/// by stance, aim, and the SUPPRESSED flag, which is `true` iff the ganger currently carries a
/// [`Suppressed`] component). A ganger with no mapped / present sprite (a Dead ganger's is
/// despawned) is skipped.
fn reframe_one(
    sprites: &GangerSprites,
    roles: &CharacterRoles,
    presenters: &mut Query<&mut Sprite, With<GangerSprite>>,
    data: (
        Entity,
        &Faction,
        &Facing,
        &Stance,
        &Aiming,
        &LifeState,
        Option<&Suppressed>,
    ),
) {
    let (entity, faction, facing, stance, aiming, life, suppressed) = data;
    let Some(presenter) = sprites.sprite_for(entity) else {
        return;
    };
    let Ok(mut sprite) = presenters.get_mut(presenter) else {
        return;
    };
    // Reframe to the facing-correct frame.
    if let Some(atlas) = sprite.texture_atlas.as_mut() {
        atlas.index = atlas_index(roles, *faction, *facing);
    }
    // Re-tint: the faction/life base, modulated by the stance + aiming + suppressed delta.
    sprite.color = stance_aiming_tint(*faction, *life, *stance, *aiming, suppressed.is_some());
}

/// `Update` (`PresenterSystems::Draw`): apply a [`Changed<LifeState>`] to a ganger
/// sprite.
///
/// A [`Downed`](LifeState::Downed) ganger's sprite is re-tinted to the downed grey-out
/// (greyed out, out of the fight); a [`Dead`](LifeState::Dead) ganger's presenter sprite
/// is DESPAWNED and its [`GangerSprites`] entry dropped (the contract's chosen
/// death-delta — despawn, not a corpse tile). An [`Alive`](LifeState::Alive) transition
/// (a revive) restores the live tint.
///
/// GTW-331 — the death-despawn DISCRIMINATOR. The sim flips a shot-killed ganger's
/// [`LifeState`] to [`Dead`](LifeState::Dead) AND emits the killing
/// [`ShotFired`](gdtf_battle_sim::ShotFired) in the SAME tick (`dispatch_fire` — `fire()`
/// applies the damage then writes the per-round signal), so the presenter observes BOTH on the
/// same drain update — well BEFORE the staggered killing tracer flies + impacts (GTW-308). Were
/// the sprite despawned here, the body would vanish at sim-drain time, before its tracer lands.
///
/// So a [`Dead`](LifeState::Dead) ganger is despawned here ONLY when NO killing shot for it
/// arrived this frame — i.e. no drained [`ShotFired`] whose threaded
/// [`HitReport`](gdtf_battle_sim::HitReport) names this ganger ([`ShotKind::Ganger`]) and left it
/// [`Dead`](LifeState::Dead). That is a NON-shot death (a bleed-out, a directly-set state — no
/// tracer is coming), so it despawns promptly. When a killing shot DID arrive, the despawn is
/// DEFERRED to [`despawn_killed_ganger_on_impact`], which drains the bolt's
/// [`ShotImpactResolved`](crate::ShotImpactResolved) signal at the staggered impact moment — so the
/// body lives until the tracer reaches it, then despawns exactly once.
///
/// `ShotFired` is the reliable discriminator (NOT a query of the in-flight bolt): the bolt is
/// spawned via a deferred `commands.spawn_scene` and its [`ProjectileTravel`](crate::fx::ProjectileTravel) materializes only on
/// a later schedule, so it is NOT queryable on the drain frame the life-state flips — but the
/// `ShotFired` message IS present that exact frame (its own reader cursor, independent of the
/// projectile spawner's). The [`GangerSprites`] entry is dropped on whichever path despawns the
/// sprite, never both (a shot-kill is guarded out here and despawned at impact; a non-shot death
/// has no impact to fire).
///
/// Param-only (`bevy-traps.md` #7): [`Commands`], [`ResMut<GangerSprites>`], the
/// changed-life query, the presenter-sprite `Sprite` query, and the
/// [`MessageReader<ShotFired>`](gdtf_battle_sim::ShotFired) the discriminator drains for this
/// frame's killing shots.
pub fn update_ganger_life_state(
    mut commands: Commands,
    mut sprites: ResMut<GangerSprites>,
    changed: Query<(Entity, &Faction, &LifeState), Changed<LifeState>>,
    mut presenters: Query<&mut Sprite, With<GangerSprite>>,
    mut shots: MessageReader<ShotFired>,
) {
    // The set of gangers a killing shot arrived for THIS frame — those deaths are pending an
    // incoming tracer, so the despawn is deferred to `despawn_killed_ganger_on_impact`. Built
    // once per run from this frame's `ShotFired` (its own reader cursor; the projectile spawner
    // drains the buffer through a separate cursor).
    let shot_killed: Vec<Entity> = shots.read().filter_map(shot_kill_victim).collect();
    for (entity, faction, life) in &changed {
        match life {
            LifeState::Dead => {
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
            LifeState::Downed | LifeState::Alive => {
                let Some(presenter) = sprites.sprite_for(entity) else {
                    continue;
                };
                let Ok(mut sprite) = presenters.get_mut(presenter) else {
                    continue;
                };
                sprite.color = ganger_tint(*faction, *life);
            }
        }
    }
}

/// The sim ganger [`Entity`] a [`ShotFired`](gdtf_battle_sim::ShotFired) KILLED, if it struck a
/// ganger ([`ShotKind::Ganger`]) AND left it [`Dead`](LifeState::Dead) (GTW-331) — else [`None`]
/// (a non-ganger hit, a non-lethal hit, a clean miss, or a geometry-only round).
///
/// [`update_ganger_life_state`]'s discriminator reads it over this frame's `ShotFired` to find
/// deaths pending a tracer; it delegates to [`report_kill_victim`] so the fire-frame guard and the
/// impact-frame despawn ([`despawn_killed_ganger_on_impact`]) classify a kill IDENTICALLY.
fn shot_kill_victim(shot: &ShotFired) -> Option<Entity> {
    report_kill_victim(shot.report.as_ref())
}

/// The sim ganger [`Entity`] a shot's [`HitReport`](gdtf_battle_sim::HitReport) KILLED, if it
/// struck a ganger ([`ShotKind::Ganger`]) AND left it [`Dead`](LifeState::Dead) (GTW-331) — else
/// [`None`].
///
/// The shared kill-classifier both death-despawn halves use over the SAME sim verdict: the
/// fire-frame guard reads it off the [`ShotFired`](gdtf_battle_sim::ShotFired)'s threaded report
/// (via [`shot_kill_victim`]), the impact-frame despawn reads it off the
/// [`ShotImpactResolved`](crate::ShotImpactResolved)'s threaded report — so a kill is the same
/// kill on both ends (the deferral and the despawn never disagree). The [`Entity`] is framework
/// plumbing (the no-bare-types carve-out).
fn report_kill_victim(report: Option<&HitReport>) -> Option<Entity> {
    let report = report?;
    let ShotKind::Ganger(victim) = report.kind else {
        return None;
    };
    matches!(
        report.applied.map(|applied| applied.life_after),
        Some(LifeState::Dead)
    )
    .then_some(victim)
}

/// `Update` (`PresenterSystems::Draw`): despawn the presenter sprite of a ganger killed by a
/// shot WHEN the killing tracer lands (GTW-331).
///
/// Drains [`MessageReader<ShotImpactResolved>`](crate::ShotImpactResolved) — the shared per-shot
/// impact-resolved signal [`animate_impact`](crate::animate_impact) emits at each staggered impact
/// (GTW-328). For every signal whose threaded
/// [`HitReport`](gdtf_battle_sim::HitReport) struck a ganger ([`ShotKind::Ganger`]) AND left it
/// [`Dead`](LifeState::Dead) ([`life_after`](gdtf_battle_sim::AppliedDamage::life_after)), it
/// despawns that ganger's mapped presenter sprite and drops its [`GangerSprites`] entry — so the
/// body vanishes the instant its killing bolt arrives, not at sim-drain time.
///
/// This is the SHOT-KILL half of the death-despawn (the NON-shot half stays in
/// [`update_ganger_life_state`], which despawns only a death with no pending tracer). The two are
/// disjoint: a shot-kill is guarded OUT of the life-state path (a tracer is pending) and despawned
/// here; a non-shot death has no impact signal and is despawned there — so the [`GangerSprites`]
/// entry is dropped EXACTLY ONCE. A signal for an already-despawned / unmapped ganger (e.g. a
/// non-lethal hit, or a corpse the life-state path already removed) is a no-op
/// ([`GangerSprites::remove`] returns [`None`]).
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

/// `Update` (`PresenterSystems::Draw`): despawn the presenter sprite of a ganger whose
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

/// `Update` (`PresenterSystems::Draw`, runs only on an [`ActiveLevel`] OR [`ViewMode`]
/// change): show the ganger sprites within the new drawn storey band, hide the rest.
///
/// On an [`ActiveLevel`](crate::ActiveLevel) change (`ActiveLevel::is_changed`) OR a
/// [`ViewMode`](crate::ViewMode) change (`ViewMode::is_changed`, GTW-521 — the full-view
/// toggle widens/narrows the band ceiling exactly as a level cycle moves it) it walks every
/// live ganger and sets its mapped presenter sprite's [`Visibility`] by whether the ganger's
/// `Position` lies WITHIN the new drawn band ([`ganger_in_drawn_band`], the shared
/// [`ActiveLevel::draws_storey`](crate::ActiveLevel::draws_storey) predicate under the current
/// [`ViewMode`] — GTW-520 C4, the SAME band the S4/S5 terrain + spawn/move sites and the fog
/// writer consult). Above-band sprites are HIDDEN (not despawned — the move / reframe systems
/// keep them current), in-band sprites (in [`ViewMode::FullView`] every storey) are SHOWN. It
/// is gated to only run when a triggering resource changed so it does no per-frame work.
///
/// Param-only (`bevy-traps.md` #7): [`Res<GangerSprites>`], [`Res<ActiveLevel>`],
/// [`Res<ViewMode>`], the ganger [`Position`] query, and the presenter-sprite [`Visibility`]
/// query.
pub fn apply_active_level_filter(
    sprites: Res<GangerSprites>,
    active: Res<ActiveLevel>,
    view: Res<ViewMode>,
    gangers: Query<(Entity, &Position)>,
    mut presenters: Query<&mut Visibility, With<GangerSprite>>,
) {
    // GTW-521: re-apply the band filter on EITHER an active-level cycle OR a view-mode toggle —
    // both change which storeys are drawn, so a stale filter would leave upper-storey gangers
    // wrongly hidden (or lower ones wrongly shown) after a FullView flip.
    if !active.is_changed() && !view.is_changed() {
        return;
    }
    for (entity, pos) in &gangers {
        let Some(presenter) = sprites.sprite_for(entity) else {
            continue;
        };
        let Ok(mut visibility) = presenters.get_mut(presenter) else {
            continue;
        };
        // GTW-520 C4 / GTW-521 C2: show a ganger anywhere in the new DRAWN band, hide only
        // one strictly above the band ceiling — the shared band predicate under the current
        // ViewMode, so this on-change re-apply agrees with spawn / move (and the fog writer)
        // exactly.
        *visibility = if ganger_in_drawn_band(pos, *active, *view) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

/// `Update` (`PresenterSystems::Draw`, runs only on a [`CharacterRoles`] change):
/// RE-INDEX every mapped ganger sprite to the freshly-reloaded atlas indices (GTW-375 C4).
///
/// When the GTW-375 [`redrive_character_roles_on_asset_event`](crate::redrive_character_roles_on_asset_event)
/// hot-reloads `character_roles.ron` it overwrites the resident [`CharacterRoles`] resource
/// (marking it changed). The atlas index a ganger draws is `faction_base + facing_frame`
/// ([`atlas_index`]) — the `faction_base` term is read STRUCTURALLY from [`CharacterRoles`],
/// so a re-saved table moves a faction's whole 4-frame actor run to a new base. The
/// per-field draw systems ([`spawn_ganger_sprites`] / [`reframe_ganger_sprites`]) only
/// re-read that base on an `Added` / `Changed` sim event, NOT on a resource change — so
/// already-spawned, idle sprites would keep their OLD index without this system.
///
/// On `CharacterRoles::is_changed` (the resource-change witness, including the one-time
/// initial resolve, which is harmless — it re-stamps the same index) it walks every live
/// ganger, looks its presenter sprite up through [`GangerSprites`], and re-stamps
/// `atlas.index = atlas_index(&roles, faction, facing)` from the CURRENT sim state — exactly
/// the [`reframe_ganger_sprites`] index recompute, but driven by the table change rather than
/// a facing change. It RE-INDEXES ONLY — it does NOT re-tint (the life / stance / aiming tint
/// is owned by [`update_ganger_life_state`] / [`reframe_ganger_sprites`] and must be preserved
/// across a character-sheet hot-reload).
///
/// Idempotent + battle-witness-free harmless: a re-index on an unchanged base re-stamps the
/// same value, so it is safe to run pre-battle / on the initial resolve (the Discovery's
/// "no battle-witness gate needed" note); the renderer registration gates it on
/// [`CharacterRoles`] existing (`bevy-traps.md` #1) so a `MinimalPlugins` headless app
/// without the table never runs it, and on `CharacterRoles::is_changed` so it does no
/// per-frame work. A [`Dead`](LifeState::Dead) ganger's sprite is already despawned, so its
/// lookup misses and is skipped.
///
/// Param-only (`bevy-traps.md` #7): [`Res<CharacterRoles>`], [`Res<GangerSprites>`], the live
/// ganger `(Entity, &Faction, &Facing)` query, and the presenter-sprite `Sprite` query.
pub fn reindex_ganger_sprites_on_character_roles_change(
    roles: Res<CharacterRoles>,
    sprites: Res<GangerSprites>,
    gangers: Query<(Entity, &Faction, &Facing)>,
    mut presenters: Query<&mut Sprite, With<GangerSprite>>,
) {
    for (entity, faction, facing) in &gangers {
        let Some(presenter) = sprites.sprite_for(entity) else {
            continue;
        };
        let Ok(mut sprite) = presenters.get_mut(presenter) else {
            continue;
        };
        // RE-INDEX ONLY — preserve the existing tint (do NOT touch `sprite.color`).
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = atlas_index(&roles, *faction, *facing);
        }
    }
}
