//! Full-map static terrain tile spawn on battle ready / view change.

use bevy::{
    camera::visibility::RenderLayers,
    ecs::template::template,
    math::primitives::Rectangle,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
};
use gdtf_battle_sim::{
    battle::BattleReady,
    occupancy::{GRID_HEIGHT, GRID_WIDTH},
    prelude::{Cell, CellLevel, Level},
};

use super::{
    active_level::{ActiveLevel, ViewMode},
    band::{drawn_band, level_band},
    restamp::StampedGraphic,
    static_map::{SpriteResolveCtx, StaticMap, graphic_name_at, i32_extent, storey_has_terrain},
    treatment::{IsolateView, StoreyViewMode},
};
use crate::{TerrainFogMaterial, cell_to_world};

/// Marker on a static terrain tile, holding its cell.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TerrainSprite {
    /// Cell and storey this tile represents.
    pub at: CellLevel,
}

#[expect(
    clippy::too_many_arguments,
    reason = "material and mesh stores plus resolve ctx are separate Bevy params"
)]
/// Despawn and respawn all terrain tiles for the visible storey band.
pub fn draw_static_battlefield(
    mut commands: Commands,
    map: StaticMap,
    resolve: SpriteResolveCtx,
    active: Res<ActiveLevel>,
    view: Res<ViewMode>,
    isolate: Res<IsolateView>,
    mut materials: ResMut<Assets<TerrainFogMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut quad: Local<Option<Handle<Mesh>>>
    ,
    mut ready: MessageReader<BattleReady>,
    existing: Query<Entity, With<TerrainSprite>>,
) {
    let ready_fired = ready.read().count() > 0;
    if !ready_fired && !active.is_changed() && !view.is_changed() && !isolate.is_changed() {
        return;
    }

    for entity in &existing {
        commands.entity(entity).despawn();
    }

    let mesh = quad
        .get_or_insert_with(|| meshes.add(Rectangle::from_size(Vec2::ONE)))
        .clone();

    let graphic_facts = map.graphic_facts();

    for level in level_band(drawn_band(*active, StoreyViewMode::new(*view, *isolate))) {
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
                let material2d = MeshMaterial2d(materials.add(material));
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
