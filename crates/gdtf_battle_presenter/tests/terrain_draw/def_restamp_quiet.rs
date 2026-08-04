use bevy::{
    app::{App, Update},
    asset::{AssetEvent, AssetServer, Assets},
    ecs::message::MessageReader,
    math::{URect, UVec2},
    prelude::{
        Deref, DerefMut, DetectChanges, DetectChangesMut, IntoScheduleConfigs, Query, Ref, Res,
        ResMut, Resource, Transform, With,
    },
};
use gdtf_battle_presenter::{PresenterSystems, TerrainFogMaterial, TerrainSprite, TopDownAtlases};
use gdtf_battle_sim::prelude::{Cell, CellLevel, Level};
use gdtf_content_families::sprites::SpriteDefRegistry;
use gdtf_test_utils::{advance_until, advance_until_resource_exists};

use super::harness::*;

const RUBBLE_DEF_V1: &str = r#"(
    source: Sheet(
        sheet: "sprites/alt_tileset_terrain.png",
        rect: (x: 32, y: 0, w: 16, h: 16),
    ),
    anchor: (x: 8, y: 8),
)"#;

const RUBBLE_DEF_V2: &str = r#"(
    source: Sheet(
        sheet: "sprites/alt_tileset_terrain.png",
        rect: (x: 48, y: 0, w: 16, h: 16),
    ),
    anchor: (x: 8, y: 8),
)"#;

#[derive(Default, Deref, DerefMut, Clone, Copy)]
struct RedirtyCount(usize);

#[derive(Default, Deref, DerefMut, Clone, Copy)]
struct StorePoked(bool);

#[derive(Default, Deref, DerefMut, Clone, Copy)]
struct ModifiedCount(usize);

#[derive(Resource, Default)]
struct QuietProbe {
    terrain_redirtied: RedirtyCount,
    store_poked:       StorePoked,
    modified_events:   ModifiedCount,
}

fn record_quiet_probe(
    terrain: Query<Ref<Transform>, With<TerrainSprite>>,
    materials: Res<Assets<TerrainFogMaterial>>,
    mut modified: MessageReader<AssetEvent<TerrainFogMaterial>>,
    mut probe: ResMut<QuietProbe>,
) {
    *probe.terrain_redirtied += terrain.iter().filter(Ref::is_changed).count();
    *probe.store_poked |= materials.is_changed();
    *probe.modified_events += modified
        .read()
        .filter(|event| matches!(event, AssetEvent::Modified { .. }))
        .count();
}

fn arm_probe(app: &mut App) {
    *app.world_mut().resource_mut::<QuietProbe>() = QuietProbe::default();
}

fn assert_quiet(app: &App, window: &str) {
    let probe = app.world().resource::<QuietProbe>();
    assert_eq!(
        *probe.terrain_redirtied, 0,
        "{window}: no drawn tile's Transform may be re-dirtied (the previous \
         registry-change redraw respawned every tile)",
    );
    assert!(
        !*probe.store_poked,
        "{window}: the Assets<TerrainFogMaterial> resource tick must stay untouched \
         (compare-before-get_mut — the restamp writes only a REAL diff)",
    );
    assert_eq!(
        *probe.modified_events, 0,
        "{window}: zero AssetEvent::Modified — no uniform re-upload for unchanged defs",
    );
}

#[test]
fn unrelated_def_change_leaves_drawn_tiles_tick_quiet() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir asset root must succeed");
    let Ok(dir) = dir else { return };
    write_sprite_def(dir.path(), "wall.spritedef.ron", CENTER_WALL_DEF);
    write_sprite_def(dir.path(), "rubble.spritedef.ron", RUBBLE_DEF_V1);

    let mut app = headless_renderer_app_at(dir.path());
    app.init_resource::<QuietProbe>();
    app.add_systems(Update, record_quiet_probe.after(PresenterSystems::Draw));
    advance_until_resource_exists::<SpriteDefRegistry>(&mut app, LOAD_SAFETY_NET);
    advance_until_resource_exists::<TopDownAtlases>(&mut app, LOAD_SAFETY_NET);

    let wall_key = CellLevel::new(Cell::new(8, 7), Level::new(0));
    draw_one_wall(&mut app, wall_key);
    let entity_before = sprite_entity_at(&mut app, wall_key);
    assert!(
        entity_before.is_some(),
        "precondition: the wall tile must be drawn"
    );
    let rect_before = sprite_rect_at(&mut app, wall_key);

    app.update();
    app.update();
    arm_probe(&mut app);
    app.update();
    assert_quiet(&app, "steady baseline frame");

    arm_probe(&mut app);
    write_sprite_def(dir.path(), "rubble.spritedef.ron", RUBBLE_DEF_V2);
    app.world()
        .resource::<AssetServer>()
        .reload("content/sprites/rubble.spritedef.ron");
    let v2_rect = URect::from_corners(UVec2::new(48, 0), UVec2::new(64, 16));
    let rebuilt = advance_until(
        &mut app,
        |app| {
            app.world()
                .get_resource::<SpriteDefRegistry>()
                .and_then(|defs| def_rect(defs, "rubble"))
                == Some(v2_rect)
        },
        LOAD_SAFETY_NET,
    );
    assert!(
        rebuilt,
        "the re-saved unrelated def must rebuild the registry (the redrive path)",
    );
    app.update();
    assert_quiet(&app, "unrelated-def rebuild window");

    arm_probe(&mut app);
    app.world_mut()
        .resource_mut::<SpriteDefRegistry>()
        .set_changed();
    app.update();
    app.update();
    assert_quiet(&app, "no-op registry touch window");

    assert_eq!(
        sprite_entity_at(&mut app, wall_key),
        entity_before,
        "an unrelated def change must not respawn the wall tile",
    );
    assert_eq!(
        sprite_rect_at(&mut app, wall_key),
        rect_before,
        "an unrelated def change must not re-target the wall tile's rect",
    );
}
