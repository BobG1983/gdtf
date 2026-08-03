//! Spawn ganger sprites and retarget tweens on drawn position changes.

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

fn ganger_sprite(appearance: GangerAppearance, atlases: &TopDownAtlases) -> Option<Sprite> {
    let chars = atlases.role(SheetRole::Characters)?;
    let mut sprite = Sprite::from_atlas_image(
        chars.image.clone(),
        TextureAtlas {
            layout: chars.layout.clone(),
            index: appearance.atlas_index,
        },
    );
    sprite.custom_size = Some(Vec2::splat(CELL_PX));
    sprite.color = appearance.tint;
    Some(sprite)
}

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

/// Spawn a presenter sprite for each newly positioned living ganger.
pub fn spawn_ganger_sprites(
    mut commands: Commands,
    mut sprites: ResMut<GangerSprites>,
    roles: Res<CharacterRoles>,
    atlases: Res<TopDownAtlases>,
    facts: GangerVisibilityFacts,
    added: Query<SpawnedGanger, Added<Position>>,
) {
    for (entity, pos, faction, facing, stance, aiming, life, suppressed) in &added {
        if matches!(life, LifeState::Dead) {
            continue;
        }
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
        let (cell, level) = pos.split();
        let visibility = facts.classify(pos, *faction, *life);
        let spawn_world = cell_to_world_layered(cell, level, Layer::Actor);
        let transform = Transform::from_translation(spawn_world);
        let layers = RenderLayers::layer(crate::WORLD_RENDER_LAYER);
        let tween = SpriteTween::settled(spawn_world);
        let presenter = commands
            .spawn_scene((
                bsn! { template(move |_| Ok(sprite.clone())) },
                template_value(transform),
                template_value(visibility),
                template_value(layers),
                bsn! { template(move |_| Ok(tween.clone())) },
            ))
            .insert(GangerSprite { entity })
            .id();
        sprites.insert(entity, presenter);
    }
}

/// Retarget sprite tweens when drawn position changes.
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
        let target = cell_to_world_layered(cell, level, Layer::Actor);
        tween.retarget(transform.translation, target);
    }
}
