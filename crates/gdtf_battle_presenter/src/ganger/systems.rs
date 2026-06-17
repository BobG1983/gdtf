//! The change-driven ganger-sprite spawn / move / reframe / life / removal / level-filter
//! systems and their per-ganger projection helpers.

use bevy::{camera::visibility::RenderLayers, ecs::lifecycle::RemovedComponents, prelude::*};
use gdtf_battle_sim::{Aiming, Cell, Facing, Faction, Level, LifeState, Position, Stance};

use super::{
    frame::atlas_index,
    roles::CharacterRoles,
    sprite_map::{GangerSprite, GangerSprites},
    tint::{ganger_tint, stance_aiming_tint},
};
use crate::{ActiveLevel, CELL_PX, SheetRole, TopDownAtlases, cell_to_world};

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

/// Whether a ganger at `pos` is on the presenter's `active` storey.
///
/// The SAME active-level filter the S4 static draw uses: compare the ganger's
/// `Position` `z` against the active [`Level`] (`bevy-traps.md` consistency with the
/// terrain filter). `active` is the dereferenced [`ActiveLevel`]'s inner [`Level`].
fn on_active_level(pos: &Position, active: Level) -> bool {
    pos.z == i32::from(*active)
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
/// ganger on the active level.
///
/// For every ganger whose [`Position`] was [`Added`] this update AND is on the
/// [`ActiveLevel`], build a [`Sprite`] (atlas index `faction_base + facing_frame`, the
/// faction tint) at [`cell_to_world`](crate::cell_to_world), on the
/// [`WORLD_RENDER_LAYER`](crate::WORLD_RENDER_LAYER), with the [`GangerSprite`] marker;
/// record `sim Entity -> presenter Entity` in [`GangerSprites`]. Off-`ActiveLevel`
/// gangers are spawned HIDDEN (a [`Visibility::Hidden`] sprite is recorded too) so a
/// later [`apply_active_level_filter`] can show it without a respawn — the level filter
/// is uniform across spawn / move / level-change.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`], [`ResMut<GangerSprites>`], the read
/// resources, and the [`Added<Position>`] ganger query.
pub fn spawn_ganger_sprites(
    mut commands: Commands,
    mut sprites: ResMut<GangerSprites>,
    roles: Res<CharacterRoles>,
    atlases: Res<TopDownAtlases>,
    active: Res<ActiveLevel>,
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
        let visibility = if on_active_level(pos, **active) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        let presenter = commands
            .spawn((
                sprite,
                Transform::from_translation(cell_to_world(cell, level)),
                visibility,
                RenderLayers::layer(crate::WORLD_RENDER_LAYER),
                GangerSprite { entity },
            ))
            .id();
        sprites.insert(entity, presenter);
    }
}

/// `Update` (`PresenterSystems::Draw`, `.after(spawn_ganger_sprites)`): move (do NOT
/// respawn) the presenter sprite of a ganger whose [`Position`] changed.
///
/// For every ganger whose [`Position`] is [`Changed`], look the presenter sprite up
/// through [`GangerSprites`] and move its [`Transform`] to the new
/// [`cell_to_world`](crate::cell_to_world), updating its [`Visibility`] by whether the
/// new `(cell, level)` is on the [`ActiveLevel`]. It does NOT spawn a second sprite: it
/// is idempotent via the map (a just-`Added` ganger handled by [`spawn_ganger_sprites`]
/// this same update is already mapped — `.after(spawn_ganger_sprites)` guarantees the
/// entry exists — so this only re-sets the same transform; a not-yet-mapped ganger is
/// skipped). The contract's "idempotent via the map" move path.
///
/// Param-only (`bevy-traps.md` #7): [`Res<GangerSprites>`], [`Res<ActiveLevel>`], the
/// moved-ganger query, and the presenter-sprite [`Transform`] / [`Visibility`] query.
pub fn move_ganger_sprites(
    sprites: Res<GangerSprites>,
    active: Res<ActiveLevel>,
    moved: Query<(Entity, &Position), Changed<Position>>,
    mut presenters: Query<(&mut Transform, &mut Visibility), With<GangerSprite>>,
) {
    for (entity, pos) in &moved {
        let Some(presenter) = sprites.sprite_for(entity) else {
            continue;
        };
        let Ok((mut transform, mut visibility)) = presenters.get_mut(presenter) else {
            continue;
        };
        let (cell, level) = cell_and_level(pos);
        transform.translation = cell_to_world(cell, level);
        *visibility = if on_active_level(pos, **active) {
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
);

/// The "any of facing / stance / aiming changed" [`QueryFilter`] driving the
/// reframe/re-tint, factored out for the same `type_complexity` reason as
/// [`ReframeData`].
///
/// [`QueryFilter`]: bevy::ecs::query::QueryFilter
type ReframeChanged = Or<(Changed<Facing>, Changed<Stance>, Changed<Aiming>)>;

/// `Update` (`PresenterSystems::Draw`): reframe / re-tint a ganger sprite whose
/// [`Facing`], [`Stance`], or [`Aiming`] changed.
///
/// For every ganger whose [`Facing`] / [`Stance`] / [`Aiming`] is [`Changed`], look the
/// presenter sprite up through [`GangerSprites`] and recompute its texture-atlas index
/// (facing reframe via the 8->4 map) and re-tint it (the stance / aiming delta) in
/// place. The reframe always recomputes from the CURRENT facing; the aiming delta
/// brightens the sprite (an aimed ganger reads "ready"); the stance delta dims a prone
/// ganger (a flattened silhouette). A [`Dead`](LifeState::Dead) ganger's sprite is
/// already despawned, so its lookup misses and is skipped.
///
/// Param-only (`bevy-traps.md` #7): [`Res<GangerSprites>`], the changed-state ganger
/// query, and the presenter-sprite [`Sprite`] query.
pub fn reframe_ganger_sprites(
    sprites: Res<GangerSprites>,
    roles: Res<CharacterRoles>,
    changed: Query<ReframeData, ReframeChanged>,
    mut presenters: Query<&mut Sprite, With<GangerSprite>>,
) {
    for (entity, faction, facing, stance, aiming, life) in &changed {
        let Some(presenter) = sprites.sprite_for(entity) else {
            continue;
        };
        let Ok(mut sprite) = presenters.get_mut(presenter) else {
            continue;
        };
        // Reframe to the facing-correct frame.
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = atlas_index(&roles, *faction, *facing);
        }
        // Re-tint: the faction/life base, modulated by the stance + aiming delta.
        sprite.color = stance_aiming_tint(*faction, *life, *stance, *aiming);
    }
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
/// Param-only (`bevy-traps.md` #7): [`Commands`], [`ResMut<GangerSprites>`], the
/// changed-life query, and the presenter-sprite [`Sprite`] query.
pub fn update_ganger_life_state(
    mut commands: Commands,
    mut sprites: ResMut<GangerSprites>,
    changed: Query<(Entity, &Faction, &LifeState), Changed<LifeState>>,
    mut presenters: Query<&mut Sprite, With<GangerSprite>>,
) {
    for (entity, faction, life) in &changed {
        match life {
            LifeState::Dead => {
                // Despawn the presenter sprite and drop its map entry.
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

/// `Update` (`PresenterSystems::Draw`, runs only on an [`ActiveLevel`] change): show the
/// ganger sprites on the new active level, hide the rest.
///
/// On an [`ActiveLevel`] change ([`ActiveLevel::is_changed`]) it walks every live ganger
/// and sets its mapped presenter sprite's [`Visibility`] by whether the ganger's
/// `Position` is on the new active level (the SAME `pos.z == **ActiveLevel` filter the
/// S4 static draw uses). Off-level sprites are HIDDEN (not despawned — the move / reframe
/// systems keep them current), on-level sprites are SHOWN. It is gated to only run when
/// the resource changed so it does no per-frame work.
///
/// Param-only (`bevy-traps.md` #7): [`Res<GangerSprites>`], [`Res<ActiveLevel>`], the
/// ganger [`Position`] query, and the presenter-sprite [`Visibility`] query.
pub fn apply_active_level_filter(
    sprites: Res<GangerSprites>,
    active: Res<ActiveLevel>,
    gangers: Query<(Entity, &Position)>,
    mut presenters: Query<&mut Visibility, With<GangerSprite>>,
) {
    if !active.is_changed() {
        return;
    }
    for (entity, pos) in &gangers {
        let Some(presenter) = sprites.sprite_for(entity) else {
            continue;
        };
        let Ok(mut visibility) = presenters.get_mut(presenter) else {
            continue;
        };
        *visibility = if on_active_level(pos, **active) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}
