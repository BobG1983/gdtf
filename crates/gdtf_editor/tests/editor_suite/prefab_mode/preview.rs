use bevy::prelude::*;
use gdtf_battle_presenter::{resolve_sprite, source_parts, source_urect};
use gdtf_battle_sim::{
    metric::{CellLevel, Level},
    prelude::Cell,
    terrain::{
        def::{TerrainDefRegistry, TerrainUuid, TerrainView},
        facing::TerrainFacing,
    },
};
use gdtf_content_families::sprites::SpriteDefRegistry;
use gdtf_editor::{
    EditorMap, EditorMode, HoveredCell, MapEditorSession, PreviewPan, PreviewTarget,
    ProposedPlacement, apply_placement,
};

use super::harness::*;

// Authored keys of the three shipped defs these cases paint. Each test asserts the
// registry holds its own before reading anything off it.
const BARRICADE: &str = "00000000-0000-0000-0000-01840a910001";
const DEBRIS_PILE: &str = "00000000-0000-0000-0000-01840a910003";
const BULKHEAD_WALL: &str = "00000000-0000-0000-0000-01840a910002";

fn sprite_count(app: &mut App) -> usize {
    app.world_mut().query::<&Sprite>().iter(app.world()).count()
}

fn uuid_of(text: &str) -> TerrainUuid {
    let Ok(parsed) = bevy::asset::uuid::Uuid::parse_str(text) else {
        unreachable!("{text} must be hyphenated UUID text")
    };
    TerrainUuid::new(parsed)
}

fn registries(app: &App) -> Option<(TerrainDefRegistry, SpriteDefRegistry)> {
    let terrain = app.world().get_resource::<TerrainDefRegistry>().cloned()?;
    let sprites = app.world().get_resource::<SpriteDefRegistry>().cloned()?;
    Some((terrain, sprites))
}

// The sheet rect drawn by the sprite key this def names for one of its views.
fn view_rect(
    terrain: &TerrainDefRegistry,
    sprites: &SpriteDefRegistry,
    key: TerrainUuid,
    view: TerrainView,
) -> Option<Rect> {
    let sprite = terrain.def(&key)?.views.sprite(view)?;
    let def = resolve_sprite(sprites, sprite)?;
    let (_path, rect) = source_parts(&def.source);
    rect.map(|rect| source_urect(rect).as_rect())
}

// The rects two def-and-view pairs draw, asserted to both resolve and to differ, so no
// comparison below passes on two untextured tiles or on one shared sprite.
fn two_view_rects(
    terrain: &TerrainDefRegistry,
    sprites: &SpriteDefRegistry,
    one: (TerrainUuid, TerrainView),
    other: (TerrainUuid, TerrainView),
) -> (Option<Rect>, Option<Rect>) {
    let first = view_rect(terrain, sprites, one.0, one.1);
    let second = view_rect(terrain, sprites, other.0, other.1);
    assert!(
        first.is_some() && second.is_some(),
        "{one:?} and {other:?} must both resolve to a sheet rect, or the comparisons below \
         pass on two untextured tiles — got {first:?} and {second:?}",
    );
    assert_ne!(
        first, second,
        "{one:?} and {other:?} must draw different rects, or nothing below can tell the two \
         cells apart",
    );
    (first, second)
}

// Every sheet rect the preview draws this frame. The preview tiles are this app's only
// sprites, which `sprite_count` above already reads them as.
fn drawn_rects(app: &mut App) -> Vec<Rect> {
    app.world_mut()
        .query::<&Sprite>()
        .iter(app.world())
        .filter_map(|sprite| sprite.rect)
        .collect()
}

// The hover ghost is the only preview tile lifted above every storey's own z.
fn ghost_rect(app: &mut App) -> Option<Rect> {
    let mut top: Option<(f32, Option<Rect>)> = None;
    let mut query = app.world_mut().query::<(&Sprite, &Transform)>();
    for (sprite, transform) in query.iter(app.world()) {
        let z = transform.translation.z;
        let higher = match top {
            None => true,
            Some((best, _)) => z > best,
        };
        if higher {
            top = Some((z, sprite.rect));
        }
    }
    top.and_then(|(_, rect)| rect)
}

fn paint(app: &mut App, cell: Cell, tile: TerrainUuid, facing: TerrainFacing) -> bool {
    let (size, theme) = {
        let Some(session) = app.world().get_resource::<MapEditorSession>() else {
            unreachable!("the session is inserted in Editing")
        };
        (session.grid_size(), session.theme())
    };
    let world = app.world_mut();
    let registry = world
        .get_resource::<TerrainDefRegistry>()
        .cloned()
        .unwrap_or_default();
    let Some(mut map) = world.get_resource_mut::<EditorMap>() else {
        unreachable!("the map is inserted in Editing")
    };
    let slot = CellLevel::new(cell, Level::new(0));
    let placement = ProposedPlacement::new(slot, tile, facing);
    apply_placement(&mut map, &registry, theme, &placement, size)
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

#[test]
fn the_hover_ghost_draws_the_view_its_paint_facing_selects() {
    let mut app = editor_app();
    advance_to_editing(&mut app);
    for _ in 0..8 {
        app.update();
    }

    let wall = uuid_of(BULKHEAD_WALL);
    let Some((terrain, sprites)) = registries(&app) else {
        unreachable!("both registries must be resident once the editor is Editing")
    };
    assert!(
        terrain.def(&wall).is_some(),
        "the shipped registry must hold {BULKHEAD_WALL}, or this case reads nothing",
    );
    let (north, east) = two_view_rects(
        &terrain,
        &sprites,
        (wall, TerrainView::Edge(TerrainFacing::North)),
        (wall, TerrainView::Edge(TerrainFacing::East)),
    );

    {
        let Some(mut session) = app.world_mut().get_resource_mut::<MapEditorSession>() else {
            unreachable!("the session is inserted in Editing");
        };
        session.select_tile(wall);
        session.set_facing(TerrainFacing::East);
    }
    {
        let Some(mut hovered) = app.world_mut().get_resource_mut::<HoveredCell>() else {
            unreachable!("the hovered cell is inserted in Editing");
        };
        hovered.set(Cell::new(3, 3), Level::new(0));
    }
    app.update();

    assert_eq!(
        ghost_rect(&mut app),
        east,
        "the ghost over an unpainted cell draws the view the paint's own facing selects — \
         building the ghost's placement at the default facing draws the north rect {north:?}",
    );
}

#[test]
fn two_painted_defs_of_one_kind_draw_their_own_sprites() {
    let mut app = editor_app();
    advance_to_editing(&mut app);
    for _ in 0..8 {
        app.update();
    }

    let barricade = uuid_of(BARRICADE);
    let debris = uuid_of(DEBRIS_PILE);
    let Some((terrain, sprites)) = registries(&app) else {
        unreachable!("both registries must be resident once the editor is Editing")
    };
    let north = TerrainView::Facing(TerrainFacing::North);
    let (barricade_rect, debris_rect) =
        two_view_rects(&terrain, &sprites, (barricade, north), (debris, north));

    assert!(
        paint(&mut app, Cell::new(1, 1), barricade, TerrainFacing::North),
        "painting the barricade on an empty ground cell must commit",
    );
    assert!(
        paint(&mut app, Cell::new(2, 1), debris, TerrainFacing::North),
        "painting the debris pile on an empty ground cell must commit",
    );
    app.update();

    let drawn = drawn_rects(&mut app);
    assert!(
        barricade_rect.is_some_and(|rect| drawn.contains(&rect)),
        "the barricade cell must draw the barricade def's own sprite — resolving every def to \
         one shared sprite leaves {barricade_rect:?} undrawn; drawn: {drawn:?}",
    );
    assert!(
        debris_rect.is_some_and(|rect| drawn.contains(&rect)),
        "the debris cell must draw the debris def's own sprite — resolving every def to one \
         shared sprite leaves {debris_rect:?} undrawn; drawn: {drawn:?}",
    );
}

#[test]
fn one_painted_def_at_two_facings_draws_two_sprites() {
    let mut app = editor_app();
    advance_to_editing(&mut app);
    for _ in 0..8 {
        app.update();
    }

    let wall = uuid_of(BULKHEAD_WALL);
    let Some((terrain, sprites)) = registries(&app) else {
        unreachable!("both registries must be resident once the editor is Editing")
    };
    let (north, east) = two_view_rects(
        &terrain,
        &sprites,
        (wall, TerrainView::Edge(TerrainFacing::North)),
        (wall, TerrainView::Edge(TerrainFacing::East)),
    );

    assert!(
        paint(&mut app, Cell::new(1, 1), wall, TerrainFacing::North),
        "painting the wall facing north must commit",
    );
    assert!(
        paint(&mut app, Cell::new(2, 1), wall, TerrainFacing::East),
        "painting the same wall facing east must commit",
    );
    app.update();

    let drawn = drawn_rects(&mut app);
    assert!(
        north.is_some_and(|rect| drawn.contains(&rect)),
        "the north-facing cell must draw the def's `Edge(North)` view; drawn: {drawn:?}",
    );
    assert!(
        east.is_some_and(|rect| drawn.contains(&rect)),
        "the east-facing cell must draw the def's `Edge(East)` view — resolving both tiles at \
         the default facing leaves {east:?} undrawn; drawn: {drawn:?}",
    );
}
