use bevy::{
    camera::visibility::RenderLayers,
    ecs::template::template,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
};
use gdtf_battle_sim::{
    armor_wear::ArmorBroken, effects::bleed::Bleeding, ganger::Wounds,
    occupancy_sync::CoverDestroyed, prelude::Position,
};

use super::{
    flash::{FlashTtl, FxFlash},
    roles::EffectRoles,
};
use crate::{CELL_PX, SheetRole, TileIndex, TopDownAtlases, cell_to_world, playback::Played};

pub(super) fn bleed_tint(wounds: Wounds) -> Color {
    let remaining = f32::from(*wounds);
    let alpha = (1.0 / (1.0 + remaining)).clamp(0.35, 1.0);
    Color::srgba(0.8, 0.05, 0.05, alpha)
}

pub(super) fn fx_sprite(index: TileIndex, tint: Color, atlases: &TopDownAtlases) -> Option<Sprite> {
    fx_sprite_scaled(index, tint, 1.0, atlases)
}

pub(super) fn fx_sprite_scaled(
    index: TileIndex,
    tint: Color,
    scale: f32,
    atlases: &TopDownAtlases,
) -> Option<Sprite> {
    let effects = atlases.role(SheetRole::Effects)?;
    let mut sprite = Sprite::from_atlas_image(
        effects.image.clone(),
        TextureAtlas {
            layout: effects.layout.clone(),
            index:  *index,
        },
    );
    sprite.custom_size = Some(Vec2::splat(CELL_PX * scale));
    sprite.color = tint;
    Some(sprite)
}

pub(super) fn spawn_flash(commands: &mut Commands, sprite: Sprite, world: Vec3) {
    let transform = Transform::from_translation(world);
    let layers = RenderLayers::layer(crate::WORLD_RENDER_LAYER);
    commands
        .spawn_scene((
            bsn! { template(move |_| Ok(sprite.clone())) },
            template_value(transform),
            template_value(layers),
            template_value(FlashTtl::new()),
        ))
        .insert(FxFlash);
}

pub fn read_bleeding(
    mut commands: Commands,
    atlases: Res<TopDownAtlases>,
    roles: Res<EffectRoles>,
    mut bleeds: MessageReader<Played<Bleeding>>,
    positions: Query<&Position>,
    wounds: Query<&Wounds>,
) {
    for msg in bleeds.read() {
        let (Ok(pos), Ok(wound)) = (positions.get(msg.ganger), wounds.get(msg.ganger)) else {
            continue;
        };
        let (cell, level) = pos.split();
        let Some(sprite) = fx_sprite(roles.bleed, bleed_tint(*wound), &atlases) else {
            continue;
        };
        spawn_flash(&mut commands, sprite, cell_to_world(cell, level));
    }
}

pub fn read_armor_broken(
    mut commands: Commands,
    atlases: Res<TopDownAtlases>,
    roles: Res<EffectRoles>,
    mut broken: MessageReader<Played<ArmorBroken>>,
    positions: Query<&Position>,
) {
    for msg in broken.read() {
        let Ok(pos) = positions.get(msg.ganger) else {
            continue;
        };
        let (cell, level) = pos.split();
        let Some(sprite) = fx_sprite(roles.armor_break, Color::WHITE, &atlases) else {
            continue;
        };
        spawn_flash(&mut commands, sprite, cell_to_world(cell, level));
    }
}

pub fn read_cover_destroyed(
    mut commands: Commands,
    atlases: Res<TopDownAtlases>,
    roles: Res<EffectRoles>,
    mut destroyed: MessageReader<Played<CoverDestroyed>>,
) {
    for msg in destroyed.read() {
        let (cell, level) = msg.at.split();
        let Some(sprite) = fx_sprite(roles.cover_destroyed, Color::WHITE, &atlases) else {
            continue;
        };
        spawn_flash(&mut commands, sprite, cell_to_world(cell, level));
    }
}
