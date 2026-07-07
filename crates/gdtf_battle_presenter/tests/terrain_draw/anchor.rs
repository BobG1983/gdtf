//! GTW-665 C2 — the authored ANCHOR is genuinely consumed: an OFF-CENTER anchor def,
//! authored into an isolated `TempDir` asset root and loaded through the REAL
//! `SpriteDefsFamily` folder walk, displaces the drawn tile's `Transform` so the
//! anchor point sits ON the cell position.

use std::path::Path;

use bevy::{app::App, ecs::message::Messages, math::Vec3, transform::components::Transform};
use gdtf_battle_presenter::{TerrainSprite, TopDownAtlases, cell_to_world};
use gdtf_battle_sim::{
    battle::BattleReady,
    cover::CoverLedger,
    occupancy::{TerrainKind, TerrainPlacement},
    prelude::{BattleInProgress, Cell, CellLevel, Level},
    surface::SurfaceGrid,
};
use gdtf_content_families::sprites::SpriteDefRegistry;
use gdtf_test_utils::advance_until_resource_exists;

use super::harness::*;

/// An off-center-anchor `wall` def: a 16×16 sheet cut whose anchor is the BOTTOM-CENTER
/// `(8, 16)` — the ground-contact point at the sprite's bottom edge, so the drawn CENTER
/// must rise by half the drawn height (+8 world units at `CELL_PX` 16).
const OFF_CENTER_WALL_DEF: &str = r#"(
    source: Sheet(
        sheet: "sprites/alt_tileset_terrain.png",
        rect: (x: 0, y: 0, w: 16, h: 16),
    ),
    anchor: (x: 8, y: 16),
)"#;

/// Materialize a minimal `content/sprites/` folder holding ONE off-center `wall` def
/// under `root` — the `TempDir` asset root the harness points the `AssetServer` at.
fn write_sprites_folder(root: &Path) {
    let dir = root.join("content").join("sprites");
    let created = std::fs::create_dir_all(&dir);
    assert!(
        created.is_ok(),
        "creating the sprites dir must succeed: {created:?}"
    );
    let written = std::fs::write(dir.join("wall.spritedef.ron"), OFF_CENTER_WALL_DEF);
    assert!(
        written.is_ok(),
        "writing the wall def must succeed: {written:?}"
    );
}

/// The one `TerrainSprite` at `key`'s `Transform.translation`, if drawn.
fn sprite_translation_at(app: &mut App, key: CellLevel) -> Option<Vec3> {
    let mut q = app.world_mut().query::<(&TerrainSprite, &Transform)>();
    q.iter(app.world())
        .find(|(t, _)| t.at == key)
        .map(|(_, transform)| transform.translation)
}

/// GTW-665 C2 — an authored OFF-CENTER anchor displaces the tile's transform: the def's
/// bottom-center anchor `(8, 16)` sits ON the cell position, so the sprite's CENTER
/// draws at `cell_to_world + (0, +8, 0)` (half the drawn height up; local pixel `y`
/// grows down, world `y` grows up).
///
/// The def is AUTHORED into a `TempDir` asset root and loaded through the REAL
/// `SpriteDefsFamily` folder walk (never an inserted fixture registry), so the pin
/// covers the authored-RON → registry → draw path end to end. With every SHIPPED seed
/// at the center anchor this offset is zero in live play (the C1 identical-pixels claim,
/// pinned by `static_draw.rs`'s exact `cell_to_world` translation assert) — this test is
/// what proves the code path genuinely READS the anchor rather than hardcoding center.
#[test]
fn off_center_anchor_displaces_the_drawn_tile_transform() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir asset root must succeed");
    let Ok(dir) = dir else { return };
    write_sprites_folder(dir.path());

    let mut app = headless_renderer_app_at(dir.path());
    advance_until_resource_exists::<SpriteDefRegistry>(&mut app, LOAD_SAFETY_NET);
    advance_until_resource_exists::<TopDownAtlases>(&mut app, LOAD_SAFETY_NET);

    let wall_cell = Cell::new(8, 7);
    let l0 = Level::new(0);
    let wall_key = CellLevel::new(wall_cell, l0);

    insert_occupancy(
        &mut app,
        vec![TerrainPlacement::new(wall_key, TerrainKind::Wall)],
    );
    app.world_mut().insert_resource(CoverLedger::new());
    app.world_mut().insert_resource(SurfaceGrid::new());
    app.world_mut().insert_resource(BattleInProgress);
    // The sim-spawned per-def fact naming the TempDir-authored off-center def.
    spawn_terrain_entity(&mut app, wall_key, "wall", None);

    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();

    // The authored def resolved through the REAL folder walk (precondition — the rect is
    // the def's, so the tile is the authored sprite, not the missing marker).
    let defs = sprite_defs(&app);
    assert!(
        defs.as_ref().is_some_and(|defs| !defs.is_empty()),
        "the TempDir-authored sprites folder must resolve into a NON-empty registry",
    );
    let Some(defs) = defs else { return };
    assert_eq!(
        sprite_rect_at(&mut app, wall_key),
        def_rect(&defs, "wall"),
        "the wall tile must draw the TempDir-authored def's rect",
    );

    // C2: the bottom-center anchor (8, 16) sits ON the cell position — the sprite CENTER
    // rises by half the drawn height: cell_to_world + (0, +8, 0).
    assert_eq!(
        sprite_translation_at(&mut app, wall_key),
        Some(cell_to_world(wall_cell, l0) + Vec3::new(0.0, 8.0, 0.0)),
        "an off-center (bottom-center) anchor must displace the drawn tile's transform by \
         half the drawn height — the anchor point sits ON the cell position (C2)",
    );
}
