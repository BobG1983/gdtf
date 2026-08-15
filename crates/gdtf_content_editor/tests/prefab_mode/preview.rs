use bevy::prelude::*;
use gdtf_battle_sim::{
    metric::{CellLevel, Level},
    prelude::Cell,
    terrain::{def::TerrainDefRegistry, facing::TerrainFacing},
};
use gdtf_content_editor::{
    EditorMap, EditorMode, HoveredCell, MapEditorSession, PreviewPan, PreviewTarget,
    ProposedPlacement, apply_placement,
};

use super::harness::*;

fn sprite_count(app: &mut App) -> usize {
    app.world_mut().query::<&Sprite>().iter(app.world()).count()
}

#[test]
fn editing_inserts_the_preview_target_and_pan() {
    let mut app = editor_app();
    advance_to_editing(&mut app);

    assert!(
        app.world().get_resource::<PreviewTarget>().is_some(),
        "the PreviewTarget render-target resource must be inserted in Editing (C4.3)",
    );
    assert!(
        app.world().get_resource::<PreviewPan>().is_some(),
        "the PreviewPan owned pan-offset target must be inserted in Editing (C4.8)",
    );
}

#[test]
fn redraw_spawns_preview_tile_sprites() {
    let mut app = editor_app();
    advance_to_editing(&mut app);

    assert_eq!(
        app.world().get_resource::<EditorMode>().copied(),
        Some(EditorMode::Prefab),
        "the editor opens in PREFAB mode",
    );
    for _ in 0..8 {
        app.update();
    }

    let sprite_count = app.world_mut().query::<&Sprite>().iter(app.world()).count();
    assert!(
        sprite_count > 0,
        "the change-driven redraw must spawn at least one preview tile sprite (the default-floor \
         fill) — a non-empty viewport, not a black one (C4.3 / C4.12); got {sprite_count}",
    );
}

#[test]
fn the_hover_ghost_needs_both_a_hovered_cell_and_a_selected_tile() {
    let mut app = editor_app();
    advance_to_editing(&mut app);
    for _ in 0..8 {
        app.update();
    }

    {
        let Some(mut session) = app.world_mut().get_resource_mut::<MapEditorSession>() else {
            unreachable!("the session is inserted in Editing");
        };
        session.clear_selected_tile();
    }
    {
        let Some(mut hovered) = app.world_mut().get_resource_mut::<HoveredCell>() else {
            unreachable!("the hovered cell is inserted in Editing");
        };
        hovered.set(Cell::new(0, 0), Level::new(0));
    }
    app.update();

    let without_ghost = sprite_count(&mut app);
    assert!(
        without_ghost > 0,
        "precondition: the redraw spawned preview tiles",
    );

    let selected = app
        .world()
        .get_resource::<MapEditorSession>()
        .and_then(MapEditorSession::default_floor);
    let Some(tile) = selected else {
        return;
    };
    {
        let Some(mut session) = app.world_mut().get_resource_mut::<MapEditorSession>() else {
            unreachable!("the session is inserted in Editing");
        };
        session.select_tile(tile);
    }
    app.update();

    assert_eq!(
        sprite_count(&mut app),
        without_ghost + 1,
        "a hovered cell plus a selected tile draws exactly one ghost sprite over the storey fill",
    );
}

#[test]
fn shared_apply_placement_paints_the_map() {
    let mut app = editor_app();
    advance_to_editing(&mut app);
    for _ in 0..8 {
        app.update();
    }

    let (tile, size, theme) = {
        let world = app.world();
        let Some(session) = world.get_resource::<MapEditorSession>() else {
            unreachable!("session inserted in Editing");
        };
        let Some(tile) = session.default_floor() else {
            assert!(world.get_resource::<EditorMap>().is_some(), "map inserted");
            return;
        };
        (tile, session.grid_size(), session.theme())
    };

    let painted = {
        let world = app.world_mut();
        let registry = world
            .get_resource::<TerrainDefRegistry>()
            .cloned()
            .unwrap_or_default();
        let Some(mut map) = world.get_resource_mut::<EditorMap>() else {
            unreachable!("map inserted in Editing");
        };
        let slot = CellLevel::new(Cell::new(1, 1), Level::new(0));
        let placement = ProposedPlacement::new(slot, tile, TerrainFacing::default());
        apply_placement(&mut map, &registry, theme, &placement, size)
    };
    assert!(
        painted,
        "a legal floor placement through the shared apply_placement must commit (C4.4 / C4.10)",
    );
    let count = app
        .world()
        .get_resource::<EditorMap>()
        .map_or(0, EditorMap::painted_count);
    assert_eq!(
        count, 1,
        "the committed placement mutates the EditorMap (one painted cell)"
    );

    app.update();
}
