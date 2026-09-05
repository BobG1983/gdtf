//! The def is authored into a `TempDir` asset root and mutated through the REAL
use bevy::{
    app::App,
    asset::{AssetId, AssetServer, Assets, Handle},
    image::Image,
    math::{URect, UVec2, Vec3},
    prelude::{MeshMaterial2d, Transform},
};
use cobalt_test_utils::{advance_until, advance_until_resource_exists};
use gdtf_battle_presenter::{TerrainFogMaterial, TerrainSprite, TopDownAtlases, cell_to_world};
use gdtf_battle_sim::prelude::{Cell, CellLevel, Level};
use gdtf_content_families::sprites::SpriteDefRegistry;

use super::harness::*;

const WALL_SHEET_V2: &str = "sprites/alt_tileset_terrain_v2.png";

fn wall_def_v2() -> String {
    format!(
        r#"(
    source: Sheet(
        sheet: "{WALL_SHEET_V2}",
        rect: (x: 16, y: 0, w: 16, h: 16),
    ),
    anchor: (x: 8, y: 16),
)"#
    )
}

fn sprite_image_at(app: &mut App, key: CellLevel) -> Option<AssetId<Image>> {
    let mut q = app
        .world_mut()
        .query::<(&TerrainSprite, &MeshMaterial2d<TerrainFogMaterial>)>();
    let handle = q
        .iter(app.world())
        .find(|(t, _)| t.at == key)
        .map(|(_, mat)| mat.id())?;
    app.world()
        .get_resource::<Assets<TerrainFogMaterial>>()?
        .get(handle)
        .map(|material| material.image.id())
}

#[test]
fn def_resave_restamps_the_drawn_tile_in_place() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir asset root must succeed");
    let Ok(dir) = dir else { return };
    write_sprite_def(dir.path(), "wall.spritedef.ron", CENTER_WALL_DEF);

    let mut app = headless_renderer_app_at(dir.path());
    advance_until_resource_exists::<SpriteDefRegistry>(&mut app);
    advance_until_resource_exists::<TopDownAtlases>(&mut app);

    let wall_cell = Cell::new(8, 7);
    let l0 = Level::new(0);
    let wall_key = CellLevel::new(wall_cell, l0);
    draw_one_wall(&mut app, wall_key);

    let v1_rect = URect::from_corners(UVec2::ZERO, UVec2::splat(16));
    assert_eq!(
        sprite_rect_at(&mut app, wall_key),
        Some(v1_rect),
        "precondition: the wall tile must draw the v1 def's origin rect",
    );
    let entity_before = sprite_entity_at(&mut app, wall_key);
    assert!(
        entity_before.is_some(),
        "precondition: the wall tile must be drawn"
    );
    let image_before = sprite_image_at(&mut app, wall_key);
    assert!(
        image_before.is_some(),
        "precondition: the drawn tile must carry a stamped material image",
    );

    write_sprite_def(dir.path(), "wall.spritedef.ron", &wall_def_v2());
    app.world()
        .resource::<AssetServer>()
        .reload("content/sprites/wall.spritedef.ron");

    let v2_rect = URect::from_corners(UVec2::new(16, 0), UVec2::new(32, 16));
    advance_until(&mut app, |app| {
        app.world()
            .get_resource::<Assets<TerrainFogMaterial>>()
            .is_some_and(|materials| {
                materials.iter().any(|(_, material)| {
                    material
                        .atlas_layout
                        .as_ref()
                        .is_some_and(|layout| layout.textures.first().copied() == Some(v2_rect))
                })
            })
    });

    assert_eq!(
        sprite_rect_at(&mut app, wall_key),
        Some(v2_rect),
        "the restamped tile must carry the v2 def's sheet rect",
    );
    let image_after = sprite_image_at(&mut app, wall_key);
    let v2_sheet: Handle<Image> = app.world().resource::<AssetServer>().load(WALL_SHEET_V2);
    assert_eq!(
        image_after,
        Some(v2_sheet.id()),
        "the restamped tile's material must carry the v2 def's re-targeted sheet image",
    );
    assert_ne!(
        image_after, image_before,
        "the sheet re-target must MOVE the stamped image handle off the v1 sheet",
    );
    let mut q = app.world_mut().query::<(&TerrainSprite, &Transform)>();
    let translation = q
        .iter(app.world())
        .find(|(t, _)| t.at == wall_key)
        .map(|(_, transform)| transform.translation);
    assert_eq!(
        translation,
        Some(cell_to_world(wall_cell, l0) + Vec3::new(0.0, 8.0, 0.0)),
        "the restamped tile's transform must re-derive from the v2 def's bottom-center anchor",
    );
    assert_eq!(
        sprite_entity_at(&mut app, wall_key),
        entity_before,
        "the restamp must keep the tile ENTITY — re-stamped in place, never despawn+respawn",
    );
}
