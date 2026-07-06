//! Sprite lifecycle-in: the `Added`-spawn and `Changed`-move / glide-retarget systems.

use bevy::{
    camera::visibility::RenderLayers,
    ecs::template::template,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
};
use gdtf_battle_sim::{Facing, Faction, LifeState, Position};

use super::{
    frame::atlas_index,
    roles::CharacterRoles,
    sprite_map::{GangerSprite, GangerSprites},
    tint::ganger_tint,
    tween::SpriteTween,
    visibility::ganger_in_drawn_band,
};
use crate::{
    ActiveLevel, CELL_PX, Layer, SheetRole, TopDownAtlases, ViewMode, cell_to_world_layered,
};

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

/// `Update` (`PresenterSystems::Scene`): spawn one presenter sprite per newly-added
/// ganger within the drawn storey band.
///
/// For every ganger whose [`Position`] was [`Added`], build a [`Sprite`] (atlas
/// index `faction_base + facing_frame`, the faction tint) at the [`Layer::Actor`](crate::Layer)
/// projection ([`cell_to_world_layered`](crate::cell_to_world_layered) — the cell's world
/// position at the ganger's OWN [`Level`](gdtf_battle_sim::Level), lifted by
/// [`GANGER_Z_BIAS`](crate::GANGER_Z_BIAS)
/// so it draws over its own floor tile, GTW-283), on the
/// [`WORLD_RENDER_LAYER`](crate::WORLD_RENDER_LAYER), with the [`GangerSprite`] marker; record
/// `sim Entity -> presenter Entity` in [`GangerSprites`].
///
/// GTW-520 (C1/C2): the sprite is spawned SHOWN when the ganger's storey lies within the drawn
/// band `0..=active` ([`ganger_in_drawn_band`], the shared
/// [`ActiveLevel::draws_storey`](crate::ActiveLevel::draws_storey) predicate) — so a ganger on
/// a LOWER storey is drawn at its own storey's Z and peeks through floor-gaps — and spawned
/// HIDDEN when it is strictly ABOVE the active level. An off-band ganger's
/// [`Visibility::Hidden`] sprite is recorded too so a later
/// [`apply_active_level_filter`](super::visibility::apply_active_level_filter) can
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
        // The canonical CellLevel::split decompose through Position's deref (GTW-565).
        let (cell, level) = pos.split();
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

/// `Update` (`PresenterSystems::Scene`, `.after(spawn_ganger_sprites)`): RE-TARGET the
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
        let (cell, level) = pos.split();
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
