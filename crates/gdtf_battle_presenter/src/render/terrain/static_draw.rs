//! Full-map static terrain tile spawn on battle ready / view change.

use bevy::{
    camera::visibility::RenderLayers,
    ecs::template::template,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
};
use gdtf_battle_sim::{
    battle::BattleReady,
    occupancy::{GRID_HEIGHT, GRID_WIDTH},
    prelude::{Cell, CellLevel, Level},
};

use super::{
    band::DrawnStoreys,
    quads::TerrainQuads,
    restamp::StampedGraphic,
    static_map::{SpriteResolveCtx, StaticMap, graphic_name_at, i32_extent, storey_has_terrain},
};
use crate::cell_to_world;

/// Marker on a static terrain tile, holding its cell.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TerrainSprite {
    /// Cell and storey this tile represents.
    pub at: CellLevel,
}

/// Despawn and respawn all terrain tiles for the visible storey band.
pub fn draw_static_battlefield(
    mut commands: Commands,
    map: StaticMap,
    resolve: SpriteResolveCtx,
    storeys: DrawnStoreys,
    mut quads: TerrainQuads,
    mut ready: MessageReader<BattleReady>,
    existing: Query<Entity, With<TerrainSprite>>,
) {
    let ready_fired = ready.read().count() > 0;
    if !ready_fired && !*storeys.changed() {
        return;
    }

    for entity in &existing {
        commands.entity(entity).despawn();
    }

    let mesh = quads.quad();

    let graphic_facts = map.graphic_facts();

    for level in storeys.levels() {
        for y in 0..i32_extent(GRID_HEIGHT) {
            for x in 0..i32_extent(GRID_WIDTH) {
                let cell = Cell::new(x, y);
                let key = CellLevel::new(cell, level);
                if let Some((_graphic, Some(footfall))) = graphic_facts.get(&key) {
                    let footfall_key: &str = footfall;
                    debug!(
                        "terrain footfall present at {key:?}: `{footfall_key}` (no \
                         footfall-audio system yet — default: silent)",
                    );
                }
                if level != Level::new(0) && !storey_has_terrain(&key, &graphic_facts, &map) {
                    continue;
                }
                let name = graphic_name_at(&key, &graphic_facts, &map);
                let (material, offset) = resolve.resolved(name, &key);
                let mesh2d = Mesh2d(mesh.clone());
                let material2d = MeshMaterial2d(quads.material(material));
                let transform =
                    Transform::from_translation(cell_to_world(cell, level) + offset.extend(0.0));
                let layers = RenderLayers::layer(crate::WORLD_RENDER_LAYER);
                commands
                    .spawn_scene((
                        bsn! { template(move |_| Ok(mesh2d.clone())) },
                        bsn! { template(move |_| Ok(material2d.clone())) },
                        template_value(transform),
                        template_value(layers),
                    ))
                    .insert((TerrainSprite { at: key }, StampedGraphic::from_key(name)));
            }
        }
    }
}
