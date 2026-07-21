//! Sprite lifecycle-in: the `Added`-spawn and `Changed`-move / glide-retarget systems.

use bevy::{
    camera::visibility::RenderLayers,
    ecs::template::template,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
};
use gdtf_battle_sim::{
    ganger::{Aiming, Facing, Suppressed},
    prelude::{Faction, LifeState, Position, Stance},
};

use super::{
    appearance::{GangerAppearance, ganger_sprite_appearance},
    roles::CharacterRoles,
    sprite_map::{GangerSprite, GangerSprites},
    tween::SpriteTween,
    visibility::GangerVisibilityFacts,
};
use crate::{
    CELL_PX, Layer, SheetRole, TopDownAtlases, cell_to_world_layered, playback::DrawnPosition,
};

/// Build one ganger [`Sprite`] on the character sheet drawn as `appearance` (the GTW-631
/// classifier's atlas index + tint), via the S3 recipe.
///
/// `Sprite::from_atlas_image(chars.image, TextureAtlas { layout, index })` with
/// `custom_size = Some(Vec2::splat(CELL_PX))` (the documented S3 sizing recipe) and the
/// classifier tint applied to `Sprite.color`. Returns [`None`] if the character sheet
/// was not loaded (so the caller skips the spawn rather than panic).
fn ganger_sprite(appearance: GangerAppearance, atlases: &TopDownAtlases) -> Option<Sprite> {
    let chars = atlases.role(SheetRole::Characters)?;
    let mut sprite = Sprite::from_atlas_image(
        chars.image.clone(),
        TextureAtlas {
            layout: chars.layout.clone(),
            index:  appearance.atlas_index,
        },
    );
    sprite.custom_size = Some(Vec2::splat(CELL_PX));
    sprite.color = appearance.tint;
    Some(sprite)
}

/// The spawn-seed ganger fields — the [`QueryData`] tuple, factored out to keep the
/// [`spawn_ganger_sprites`] query under the `type_complexity` clippy gate.
///
/// [`QueryData`]: bevy::ecs::query::QueryData
type SpawnedGanger = (
    Entity,
    &'static Position,
    &'static Faction,
    &'static Facing,
    &'static Stance,
    &'static Aiming,
    &'static LifeState,
    Option<&'static Suppressed>,
);

/// `Update` (`PresenterSystems::Scene`): spawn one presenter sprite per newly-added
/// ganger within the drawn storey band.
///
/// For every ganger whose [`Position`] was [`Added`], build a [`Sprite`] (its atlas
/// index + tint seeded through the ONE GTW-631 appearance classifier,
/// `ganger_sprite_appearance` — the same verdict the appearance resolver
/// re-stamps on every later change) at the [`Layer::Actor`](crate::Layer)
/// projection ([`cell_to_world_layered`](crate::cell_to_world_layered) — the cell's world
/// position at the ganger's OWN [`Level`](gdtf_battle_sim::metric::Level), lifted by
/// [`GANGER_Z_BIAS`](crate::GANGER_Z_BIAS)
/// so it draws over its own floor tile, GTW-283), on the
/// [`WORLD_RENDER_LAYER`](crate::WORLD_RENDER_LAYER), with the [`GangerSprite`] marker; record
/// `sim Entity -> presenter Entity` in [`GangerSprites`].
///
/// GTW-627 (C3): the sprite's INITIAL [`Visibility`] is seeded through the ONE pure
/// classifier (`GangerVisibilityFacts::classify` — the band fact AND, when the fog
/// resources are resident, the fog fact), the same decision the
/// [`resolve_ganger_visibility`](super::visibility::resolve_ganger_visibility) resolver
/// re-applies every frame. Seeding the composed verdict (not a band-only guess) means the
/// deferred `spawn_scene` materializes with the correct flag — no first-frame flicker
/// while the resolver has not yet seen the sprite. A hidden ganger's sprite is recorded in
/// the map too, so the resolver can show it later without a respawn.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`], [`ResMut<GangerSprites>`], the read
/// resources (the classifier inputs bundled as [`GangerVisibilityFacts`]), and the
/// [`Added<Position>`] ganger query.
pub fn spawn_ganger_sprites(
    mut commands: Commands,
    mut sprites: ResMut<GangerSprites>,
    roles: Res<CharacterRoles>,
    atlases: Res<TopDownAtlases>,
    facts: GangerVisibilityFacts,
    added: Query<SpawnedGanger, Added<Position>>,
) {
    for (entity, pos, faction, facing, stance, aiming, life, suppressed) in &added {
        // A Dead ganger added directly (no live frame) draws no sprite.
        if matches!(life, LifeState::Dead) {
            continue;
        }
        // GTW-631 C2: the seed IS the classifier's verdict — the same (atlas index, tint)
        // decision the appearance resolver re-stamps on every later change, so a second
        // appearance derivation is unrepresentable.
        let appearance = ganger_sprite_appearance(
            *faction,
            *facing,
            *stance,
            *aiming,
            *life,
            suppressed.is_some(),
            &roles,
        );
        let Some(sprite) = ganger_sprite(appearance, &atlases) else {
            continue;
        };
        // The canonical CellLevel::split decompose through Position's deref (GTW-565).
        let (cell, level) = pos.split();
        // GTW-627 C3: seed the sprite's initial Visibility through the ONE classifier —
        // the same band × fog verdict the resolver re-applies every frame. The Actor-layer
        // projection below uses the ganger's OWN `level`, so a lower-storey ganger draws
        // at its own storey's Z and occludes correctly (GTW-520).
        let visibility = facts.classify(pos, *faction, *life);
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
        // appearance systems and the GTW-627 visibility resolver (`resolve_ganger_visibility`,
        // which owns this sprite's `Visibility` from here on) look the sprite up through
        // the map and gracefully skip until its components exist.
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
/// movement tween (do NOT respawn, do NOT snap) of a ganger whose [`Position`] changed.
///
/// For every ganger whose [`DrawnPosition`] is [`Changed`] — the cell the playback cursor
/// has SHOWN it at, not the cell the sim has already moved it to (GTW-727 C17) — look the
/// presenter sprite up
/// through [`GangerSprites`] and RE-TARGET its [`SpriteTween`] (GTW-359 C4) — source = the
/// sprite's CURRENT (possibly mid-glide) [`Transform`] translation, target = the new
/// [`Layer::Actor`](crate::Layer) projection of the cell
/// ([`cell_to_world_layered`](crate::cell_to_world_layered) — so the
/// [`GANGER_Z_BIAS`](crate::GANGER_Z_BIAS) lift holds across moves), restarting the
/// glide clock. The actual [`Transform`] write is the
/// [`advance_sprite_tweens`](super::advance_sprite_tweens) glide that runs
/// `.after` this; setting the source to the live translation means a sim that outruns
/// the tween keeps the sprite gliding continuously toward the latest cell — it NEVER
/// snaps and NEVER gates the sim (the sim's [`Position`] is authoritative; the tween
/// only smooths the view). Covers planar AND cross-storey moves (the GENERAL per-step
/// glide that subsumes GTW-361).
///
/// It writes NO [`Visibility`](bevy::prelude::Visibility): the cross-storey show/hide
/// handoff is the resolver's
/// ([`resolve_ganger_visibility`](super::visibility::resolve_ganger_visibility), the one
/// visibility writer — GTW-627 C3), which re-classifies the moved ganger in the `Compose`
/// stage of the SAME update.
///
/// It does NOT spawn a second sprite: it is idempotent via the map (a just-`Added` ganger
/// handled by [`spawn_ganger_sprites`] this same update is already mapped —
/// `.after(spawn_ganger_sprites)` guarantees the entry + its seeded [`SpriteTween`] exist
/// — so this only re-targets the same tween; a not-yet-mapped ganger is skipped). The
/// contract's "idempotent via the map" move path.
///
/// Param-only (`bevy-traps.md` #7): [`Res<GangerSprites>`], the moved-ganger query, and
/// the presenter-sprite [`Transform`] / [`SpriteTween`] query.
pub fn move_ganger_sprites(
    sprites: Res<GangerSprites>,
    moved: Query<(Entity, &DrawnPosition), Changed<DrawnPosition>>,
    mut presenters: Query<(&Transform, &mut SpriteTween), With<GangerSprite>>,
) {
    for (entity, drawn) in &moved {
        let pos = drawn.position();
        let Some(presenter) = sprites.sprite_for(entity) else {
            continue;
        };
        let Ok((transform, mut tween)) = presenters.get_mut(presenter) else {
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
    }
}
