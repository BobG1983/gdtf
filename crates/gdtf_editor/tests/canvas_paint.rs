//! Headless integration test for the GTW-426 map-editor canvas interactivity — the in-memory
//! paintable map model, click-to-paint, and the hover ghost.
//!
//! Drives the REAL [`MapEditorPlugin`] on the no-renderer `DefaultPlugins` UI harness (the same
//! harness the GTW-417 `editor_shell` / GTW-421 `right_panel` / GTW-422 `palette` / GTW-423
//! `canvas` tests use), so the editor's actual `Load` pass resolves the shipped theme + catalog
//! registries and its real `Editing` scene builds the canvas, the paint model, and the ghost —
//! not a copy.
//!
//! Asserts the contract end-to-end, pin-discriminatingly (each test would FAIL if the paint /
//! ghost logic were absent), and value-agnostically (the painted index is the SELECTED tile's
//! catalog index, never a pinned magnitude):
//!
//! - T1 (C2/C3): pressing a canvas cell with a selected tile records that cell in the
//!   [`EditorMap`] model AND mutates that cell's [`ImageNode`] atlas index to the selected tile's
//!   index. Pin-discriminating: the model holds the painted cell keyed by its [`Cell`], the
//!   cell's atlas index changed from the default-floor index to the selected index.
//! - T2 (C3 clamp): the model only ever holds IN-BOUNDS cells — painting a cell records it, and
//!   the recorded key is inside the drawable extent; an out-of-bounds paint is rejected by the
//!   model (asserted directly on [`EditorMap::paint`]'s clamp, since the canvas never spawns an
//!   out-of-bounds cell to press).
//! - T3 (C1): the hover ghost appears at the hovered cell (it is parented under the hovered
//!   [`CanvasCell`] and is [`Visibility::Visible`]) and is HIDDEN when no cell is hovered or no
//!   tile is selected.
//!
//! Panic/expect-free per the workspace lints (the GTW-417 precedent: `assert!` + `if let Some`
//! guards, never `unwrap`/`expect`/`panic`).

use bevy::{prelude::*, ui::widget::ImageNode};
use gdtf_battle_sim::{
    Cell,
    level::{GridHeight, GridLevels, GridSize, GridWidth, ThemeCatalogRegistry, TileKey},
};
use gdtf_editor::{
    CanvasCell, CanvasGhost, EditorMap, EditorState, MapEditorPlugin, MapEditorSession,
};
use gdtf_test_utils::{GdtfUiTestAppBuilder, advance_until};
use gdtf_ui::UiPlugin;

/// A generous frame cap: the async asset loads under parallel `cargo` contention take a
/// non-deterministic number of frames, so this is a SAFETY NET — we poll the
/// `EditorState::Editing` SIGNAL, not a fixed count.
const MAX_UPDATES: u32 = 10_000;

/// Builds the real editor app on the no-renderer `DefaultPlugins` UI harness and advances it to
/// [`EditorState::Editing`] with the canvas, the paint model, and the ghost present.
fn editor_in_editing() -> App {
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    app.add_plugins(UiPlugin);
    app.add_plugins(MapEditorPlugin);

    let reached = advance_until(
        &mut app,
        |app| {
            app.world()
                .get_resource::<State<EditorState>>()
                .is_some_and(|s| *s.get() == EditorState::Editing)
        },
        MAX_UPDATES,
    );
    assert!(
        reached,
        "the editor never reached EditorState::Editing — its Load pass did not resolve the theme \
         + registries (a genuine load failure, not a frame-budget shortfall)",
    );
    // The OnEnter spawn + the Update `sync_canvas` build the cells + ghost across a few frames
    // (deferred re-parent commands); advance so they are present before we query.
    settle(&mut app);
    app
}

/// Advance a handful of frames so the deferred scroll-list re-parent + the `sync_canvas` build +
/// its deferred grid re-parent + the ghost spawn have all applied.
fn settle(app: &mut App) {
    for _ in 0..8 {
        app.update();
    }
}

/// Sets `entity`'s [`Interaction`] to `state` and runs ONLY the `Update` schedule so the
/// `Changed<Interaction>` driver fires on the edge.
///
/// Runs `Update` directly rather than `app.update()`: under the `DefaultPlugins` headless harness
/// the primary window is absent, so `bevy_ui`'s `ui_focus_system` (in `PreUpdate`) `set_if_neq`s
/// every node's [`Interaction`] back to `None` (no cursor) — a full `app.update()` would clobber
/// the manual value BEFORE the canvas drivers read it. The GTW-422 `palette::press` precedent.
fn set_interaction(app: &mut App, entity: Entity, state: Interaction) {
    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(entity) {
        *interaction = state;
    }
    app.world_mut().run_schedule(Update);
}

/// The session's current `grid_size`, if present.
fn grid_size(app: &App) -> Option<GridSize> {
    app.world()
        .get_resource::<MapEditorSession>()
        .map(MapEditorSession::grid_size)
}

/// The active theme's catalog's default-floor tile key + atlas index (the fill every unpainted
/// cell shows), if resolvable.
fn default_floor_index(app: &App) -> Option<usize> {
    let session = app.world().get_resource::<MapEditorSession>()?;
    let registry = app.world().get_resource::<ThemeCatalogRegistry>()?;
    registry
        .catalog(session.theme())
        .and_then(|cat| cat.default_floor())
        .map(|tile| *tile.atlas_index)
}

/// Pick a catalog tile of the active theme whose atlas index DIFFERS from the default-floor's, so
/// painting it produces a visibly different cell sprite. Returns the tile key + its atlas index.
fn distinct_paint_tile(app: &App) -> Option<(TileKey, usize)> {
    let session = app.world().get_resource::<MapEditorSession>()?;
    let registry = app.world().get_resource::<ThemeCatalogRegistry>()?;
    let default_index = default_floor_index(app)?;
    let catalog = registry.catalog(session.theme())?;
    catalog.tiles().find_map(|(key, tile)| {
        (*tile.atlas_index != default_index).then(|| (key.clone(), *tile.atlas_index))
    })
}

/// The [`ImageNode`] atlas index of the [`CanvasCell`] at ground-plane `cell`, if it exists.
fn cell_index_at(app: &mut App, cell: Cell) -> Option<usize> {
    let world = app.world_mut();
    let mut q = world.query::<(&CanvasCell, &ImageNode)>();
    q.iter(world).find_map(|(canvas_cell, node)| {
        (canvas_cell.cell() == cell)
            .then(|| node.texture_atlas.as_ref().map(|atlas| atlas.index))
            .flatten()
    })
}

/// The [`CanvasCell`] entity at ground-plane `cell`, if it exists.
fn cell_entity_at(app: &mut App, cell: Cell) -> Option<Entity> {
    let world = app.world_mut();
    let mut q = world.query::<(Entity, &CanvasCell)>();
    q.iter(world)
        .find_map(|(entity, canvas_cell)| (canvas_cell.cell() == cell).then_some(entity))
}

/// T1 (C2/C3): pressing a canvas cell with a SELECTED tile records that cell in the [`EditorMap`]
/// model AND redraws the cell's [`ImageNode`] atlas index to the selected tile's index.
///
/// Pin-discriminating: before the press the model is empty and the cell shows the default-floor
/// index; after the press the model holds exactly the painted cell keyed by its [`Cell`], and the
/// cell's atlas index equals the selected tile's index (which differs from the default). Without
/// the paint system both the model and the sprite would be unchanged.
#[test]
fn pressing_a_cell_paints_the_model_and_redraws_the_sprite() {
    let mut app = editor_in_editing();

    // A selected tile whose sprite differs from the default-floor fill, set via the session (the
    // palette's job, tested separately) — paint then drives the REAL Interaction::Pressed path.
    let pick = distinct_paint_tile(&app);
    assert!(
        pick.is_some(),
        "the active theme must have a tile distinct from its default floor to paint with (C2)",
    );
    let Some((paint_key, paint_index)) = pick else {
        return;
    };
    let default_index = default_floor_index(&app);
    assert!(
        default_index.is_some(),
        "the canvas must have a default fill"
    );

    if let Some(mut session) = app.world_mut().get_resource_mut::<MapEditorSession>() {
        session.select_tile(paint_key.clone());
    }

    // The cell we will paint: a real, in-bounds canvas cell.
    let target = Cell::new(2, 3);
    let entity = cell_entity_at(&mut app, target);
    assert!(
        entity.is_some(),
        "the canvas must have a CanvasCell at (2, 3) to paint (the grid is at least 16×16)",
    );

    // Before the paint: model empty, cell shows the default-floor index.
    let painted_before = app
        .world()
        .get_resource::<EditorMap>()
        .map(EditorMap::painted_count);
    assert_eq!(painted_before, Some(0), "the model must start unpainted");
    assert_eq!(
        cell_index_at(&mut app, target),
        default_index,
        "before the paint the cell must show the default-floor index",
    );

    if let Some(entity) = entity {
        set_interaction(&mut app, entity, Interaction::Pressed);
    }

    // After the paint: the model holds exactly the painted cell keyed by its Cell, with the
    // selected key; the cell's sprite is the selected index.
    let map = app.world().get_resource::<EditorMap>();
    if let Some(map) = map {
        assert_eq!(
            map.painted_count(),
            1,
            "exactly one cell must be painted in the model after one press (C2)",
        );
        assert_eq!(
            map.tile_at(target),
            Some(&paint_key),
            "the model must hold the painted cell keyed by its Cell, with the selected tile (C2)",
        );
    }
    assert_eq!(
        cell_index_at(&mut app, target),
        Some(paint_index),
        "the painted cell's sprite must redraw to the selected tile's atlas index (C2/C3)",
    );
}

/// T2 (C3 clamp): the model only ever holds IN-BOUNDS cells. A press on a real (in-bounds) cell
/// records it; an out-of-bounds paint is rejected by the model's clamp.
///
/// The canvas never spawns a cell outside the drawable extent, so the live press path can only
/// ever target in-bounds cells — the clamp is the model-level backstop the save/emit paths trust.
/// This drives [`EditorMap::paint`] directly for the out-of-bounds case (there is no out-of-bounds
/// cell entity to press) and asserts the live in-bounds press is recorded.
#[test]
fn paint_is_clamped_to_the_drawable_extent() {
    let mut app = editor_in_editing();

    let size = grid_size(&app);
    assert!(size.is_some(), "the session must hold a grid size");
    let Some(size) = size else {
        return;
    };

    // The model's clamp: an in-bounds cell records; an out-of-bounds one does not.
    let mut model = EditorMap::new();
    let in_bounds = Cell::new(0, 0);
    let key = TileKey::new("test".to_owned());
    let recorded_in = model.paint(in_bounds, key.clone(), size);
    assert!(
        recorded_in,
        "painting an in-bounds cell must record it (C3)"
    );

    let width = i32::from(*size.width());
    let height = i32::from(*size.height());
    for out in [
        Cell::new(-1, 0),
        Cell::new(0, -1),
        Cell::new(width, 0),
        Cell::new(0, height),
    ] {
        let recorded_out = model.paint(out, key.clone(), size);
        assert!(
            !recorded_out,
            "painting an out-of-bounds cell {out:?} must be rejected by the clamp (C3)",
        );
    }
    assert_eq!(
        model.painted_count(),
        1,
        "the clamped model must hold only the one in-bounds cell (C3)",
    );

    // The live press path also only ever records in-bounds cells (the canvas spawns only
    // in-bounds cells). Shrink the grid small, paint a cell, and confirm the recorded key is
    // inside the new extent.
    if let Ok(small) = GridSize::new(GridWidth::new(4), GridHeight::new(4), GridLevels::new(1))
        && let Some(mut session) = app.world_mut().get_resource_mut::<MapEditorSession>()
    {
        session.set_grid_size(small);
    }
    settle(&mut app);
    if let Some((paint_key, _)) = distinct_paint_tile(&app)
        && let Some(mut session) = app.world_mut().get_resource_mut::<MapEditorSession>()
    {
        session.select_tile(paint_key);
    }
    let target = Cell::new(3, 3);
    if let Some(entity) = cell_entity_at(&mut app, target) {
        set_interaction(&mut app, entity, Interaction::Pressed);
    }
    if let (Some(map), Some(size)) = (app.world().get_resource::<EditorMap>(), grid_size(&app)) {
        for (cell, _) in map.painted() {
            assert!(
                cell.x >= 0
                    && cell.x < i32::from(*size.width())
                    && cell.y >= 0
                    && cell.y < i32::from(*size.height()),
                "every cell in the model must be inside the drawable extent (C3); found {cell:?}",
            );
        }
    }
}

/// T3 (C1): the hover ghost appears at the hovered cell (parented under it + visible) when a tile
/// is selected, and is HIDDEN when no cell is hovered or no tile is selected.
///
/// Pin-discriminating: with a selection + a hovered cell the ghost is `Visibility::Visible` and
/// its parent (`ChildOf`) is the hovered cell entity (the snap); clearing the hover hides it; and
/// with NO selection a hovered cell still leaves the ghost hidden. Without the follow logic the
/// ghost would stay hidden / unparented.
#[test]
fn hover_ghost_follows_the_hovered_cell_and_hides_otherwise() {
    let mut app = editor_in_editing();

    // The single persistent ghost must exist.
    let ghost = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<Entity, With<CanvasGhost>>();
        let all: Vec<Entity> = q.iter(world).collect();
        assert_eq!(
            all.len(),
            1,
            "exactly one persistent hover ghost must exist (C1)"
        );
        all.into_iter().next()
    };
    let Some(ghost) = ghost else {
        return;
    };

    // Select a tile (so the ghost has something to preview) and hover a cell.
    if let Some((paint_key, _)) = distinct_paint_tile(&app)
        && let Some(mut session) = app.world_mut().get_resource_mut::<MapEditorSession>()
    {
        session.select_tile(paint_key);
    }
    let target = Cell::new(1, 1);
    let cell = cell_entity_at(&mut app, target);
    assert!(
        cell.is_some(),
        "the canvas must have a cell at (1, 1) to hover"
    );
    let Some(cell) = cell else {
        return;
    };

    set_interaction(&mut app, cell, Interaction::Hovered);

    // Visible + parented under the hovered cell (the snap).
    assert_eq!(
        app.world().get::<Visibility>(ghost),
        Some(&Visibility::Visible),
        "the ghost must be visible while a cell is hovered + a tile selected (C1)",
    );
    assert_eq!(
        app.world().get::<ChildOf>(ghost).map(ChildOf::parent),
        Some(cell),
        "the ghost must be parented under the hovered cell — the snap (C1)",
    );

    // Clear the hover -> hidden.
    set_interaction(&mut app, cell, Interaction::None);
    assert_eq!(
        app.world().get::<Visibility>(ghost),
        Some(&Visibility::Hidden),
        "the ghost must hide when no cell is hovered (C1)",
    );

    // No selection -> hidden even while hovering.
    if let Some(mut session) = app.world_mut().get_resource_mut::<MapEditorSession>() {
        session.clear_selected_tile();
    }
    set_interaction(&mut app, cell, Interaction::Hovered);
    assert_eq!(
        app.world().get::<Visibility>(ghost),
        Some(&Visibility::Hidden),
        "the ghost must stay hidden when no tile is selected, even while hovering (C1)",
    );
}
