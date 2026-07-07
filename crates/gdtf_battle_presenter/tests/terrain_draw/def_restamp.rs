//! GTW-666 — sprite-def hot-reload RESTAMP, the in-place half: a def re-save
//! re-resolves the drawn tile IN PLACE — the SAME entity carries the new
//! texture/rect and its transform re-derives from the new anchor (the tick-quiet
//! half lives in `def_restamp_quiet.rs`).
//!
//! The def is authored into a `TempDir` asset root and mutated through the REAL
//! reload path: overwrite the member file on disk, then `AssetServer::reload`
//! (the deterministic file-watcher stand-in — it re-reads the bytes off disk and
//! fires the `AssetEvent::Modified` the family redrive rebuilds the
//! `SpriteDefRegistry` from), never a hand-inserted fixture registry.

use bevy::{
    app::App,
    asset::{AssetId, AssetServer, Assets, Handle},
    image::Image,
    math::{URect, UVec2, Vec3},
    prelude::{MeshMaterial2d, Transform},
};
use gdtf_battle_presenter::{TerrainFogMaterial, TerrainSprite, TopDownAtlases, cell_to_world};
use gdtf_battle_sim::prelude::{Cell, CellLevel, Level};
use gdtf_content_families::sprites::SpriteDefRegistry;
use gdtf_test_utils::{advance_until, advance_until_resource_exists};

use super::harness::*;

/// The DIFFERENT sheet path the re-saved def RE-TARGETS — a first-class
/// SPRITE-mode edit (C1 "re-applies texture"). The `AssetServer` mints one
/// stable handle per path (whether or not the png exists under the `TempDir`
/// root), so the texture assert keys on the material's handle re-resolving to
/// this path — never on decoded pixels.
const WALL_SHEET_V2: &str = "sprites/alt_tileset_terrain_v2.png";

/// The `wall` def as RE-SAVED (over the harness's [`CENTER_WALL_DEF`] v1): a
/// DIFFERENT sheet ([`WALL_SHEET_V2`]), a different sheet rect, AND a
/// bottom-center anchor `(8, 16)` — so the restamp must re-apply all three
/// stamped thirds: the material's image (the sheet re-target), its rect, and
/// the tile's transform (+8 world-y: the ground-contact point sits ON the cell
/// position).
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

/// The stamped material's IMAGE id for the tile at `key` — the TEXTURE third of
/// the C3(a) probe, read through the SAME material the stamp seam writes
/// (single consumer, so it stays local to this suite per module-layout Rule 6).
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

/// GTW-666 C1/C3(a) — a re-saved def RESTAMPS the drawn tile IN PLACE: after the
/// member file is overwritten and reloaded (the real redrive path), the SAME tile
/// entity carries the def's NEW sheet rect and its transform re-derives from the
/// NEW anchor — mutate-not-respawn.
///
/// RED before GTW-666: the def change triggered `draw_static_battlefield`'s
/// despawn-all + respawn, so the rect/anchor updated but the ENTITY was replaced —
/// the same-entity assert below observed the respawn.
#[test]
fn def_resave_restamps_the_drawn_tile_in_place() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir asset root must succeed");
    let Ok(dir) = dir else { return };
    write_sprite_def(dir.path(), "wall.spritedef.ron", CENTER_WALL_DEF);

    let mut app = headless_renderer_app_at(dir.path());
    advance_until_resource_exists::<SpriteDefRegistry>(&mut app, LOAD_SAFETY_NET);
    advance_until_resource_exists::<TopDownAtlases>(&mut app, LOAD_SAFETY_NET);

    let wall_cell = Cell::new(8, 7);
    let l0 = Level::new(0);
    let wall_key = CellLevel::new(wall_cell, l0);
    draw_one_wall(&mut app, wall_key);

    // Precondition: the tile draws the v1 def — origin rect, centered (zero anchor offset).
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

    // The RE-SAVE: overwrite the member on disk, then reload it through the real
    // asset path (the file-watcher stand-in) — the family redrive rebuilds the
    // registry and the GTW-666 restamp re-resolves the drawn tile from it.
    write_sprite_def(dir.path(), "wall.spritedef.ron", &wall_def_v2());
    app.world()
        .resource::<AssetServer>()
        .reload("content/sprites/wall.spritedef.ron");

    let v2_rect = URect::from_corners(UVec2::new(16, 0), UVec2::new(32, 16));
    let reached = advance_until(
        &mut app,
        |app| {
            // Read the drawn rect through an immutable probe: the registry rebuild +
            // restamp both landed once the tile's material carries the v2 rect.
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
        },
        LOAD_SAFETY_NET,
    );
    assert!(
        reached,
        "the re-saved def must reach the drawn tile's material (registry rebuild + restamp)",
    );

    assert_eq!(
        sprite_rect_at(&mut app, wall_key),
        Some(v2_rect),
        "the restamped tile must carry the v2 def's sheet rect",
    );
    // The TEXTURE third (C3(a) / C1 "re-applies texture"): the v2 def re-targets
    // its SOURCE PATH, so the stamped material's image must re-resolve to the NEW
    // sheet's path-keyed handle — asserted through the restamp seam on the drawn
    // tile, not the draw-spawn path.
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
    // The v2 anchor is bottom-center (8, 16): the sprite CENTER rises half the drawn
    // height, so the transform re-derives to cell_to_world + (0, +8, 0).
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
    // Mutate-not-respawn: the SAME entity was restamped in place (RED pre-GTW-666 —
    // the registry-change redraw despawned + respawned every tile).
    assert_eq!(
        sprite_entity_at(&mut app, wall_key),
        entity_before,
        "the restamp must keep the tile ENTITY — re-stamped in place, never despawn+respawn",
    );
}
