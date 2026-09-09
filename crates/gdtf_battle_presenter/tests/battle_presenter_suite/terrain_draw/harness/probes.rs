//! What a drawn cell carries: its material rect, its stamp, its entity, and the counts.

use bevy::{app::App, asset::Assets, ecs::entity::Entity, math::URect, prelude::MeshMaterial2d};
use gdtf_battle_presenter::{
    StampedGraphic, TerrainFogMaterial, TerrainSprite, source_parts, source_urect,
};
use gdtf_battle_sim::prelude::{CellLevel, Level};
use gdtf_content_families::sprites::{SpriteDefRegistry, SpriteName};

pub(crate) fn sprite_defs(app: &App) -> Option<SpriteDefRegistry> {
    app.world().get_resource::<SpriteDefRegistry>().cloned()
}

pub(crate) fn def_rect(defs: &SpriteDefRegistry, name: &str) -> Option<URect> {
    let def = defs.def(&SpriteName::new(name.to_owned()))?;
    let (_path, rect) = source_parts(&def.source);
    rect.map(source_urect)
}

pub(crate) fn sprite_rect_at(app: &mut App, key: CellLevel) -> Option<URect> {
    let mut q = app
        .world_mut()
        .query::<(&TerrainSprite, &MeshMaterial2d<TerrainFogMaterial>)>();
    let handle = q
        .iter(app.world())
        .find(|(t, _)| t.at == key)
        .map(|(_, mat)| mat.id())?;
    let material = app
        .world()
        .get_resource::<Assets<TerrainFogMaterial>>()?
        .get(handle)?;
    material
        .atlas_layout
        .as_ref()
        .and_then(|layout| layout.textures.get(material.atlas_index).copied())
}

/// What a cell's tile is currently stamped with.
pub(crate) fn stamped_graphic_at(app: &mut App, key: CellLevel) -> Option<StampedGraphic> {
    let mut q = app.world_mut().query::<(&TerrainSprite, &StampedGraphic)>();
    q.iter(app.world())
        .find(|(t, _)| t.at == key)
        .map(|(_, stamped)| stamped.clone())
}

pub(crate) fn sprite_entity_at(app: &mut App, key: CellLevel) -> Option<Entity> {
    let mut q = app.world_mut().query::<(Entity, &TerrainSprite)>();
    q.iter(app.world())
        .find(|(_, t)| t.at == key)
        .map(|(entity, _)| entity)
}

pub(crate) fn terrain_sprite_count_on_level(app: &mut App, level: Level) -> usize {
    let z = i32::from(*level);
    let mut q = app.world_mut().query::<&TerrainSprite>();
    q.iter(app.world()).filter(|t| t.at.z == z).count()
}
