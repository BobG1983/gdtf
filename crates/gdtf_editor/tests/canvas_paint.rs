//! Headless integration test for the map-editor canvas interactivity — the in-memory paintable
//! map model, click-to-paint, and the hover ghost — swept onto the UUID-keyed terrain model
//! (GTW-495).
//!
//! Drives the REAL [`MapEditorPlugin`] on the no-renderer `DefaultPlugins` UI harness, so the
//! editor's actual `Load` pass resolves the shipped UUID-keyed registries and its real `Editing`
//! scene builds the canvas, the paint model, and the ghost — not a copy.
//!
//! Asserts the contract end-to-end, pin-discriminatingly, and value-agnostically (the painted
//! index is the SELECTED terrain's resolved index, never a pinned magnitude):
//!
//! - T1 (C2/C3): pressing a canvas cell with a selected tile records that cell in the
//!   [`EditorMap`] model AND mutates that cell's [`ImageNode`] atlas index to the selected
//!   terrain's resolved index.
//! - T2 (C3 clamp): the model only ever holds IN-BOUNDS cells.
//! - T3 (C1): the hover ghost appears at the hovered cell and is HIDDEN otherwise.
//! - T4/T5 (GTW-430 C2/C3): the hover-ghost preview tints RED over an ILLEGAL placement (a slab
//!   over a ladder) and the normal preview over a LEGAL one — driven through the live
//!   [`follow_hover_ghost`] + the SHARED `evaluate_placement`. T4 SEEDS a ladder def into the live
//!   [`TerrainDefRegistry`] (shipped content authors none) and a ladder cell into the model, then
//!   hovers it with a slab selected.
//!
//! Panic/expect-free per the workspace lints.

use bevy::{prelude::*, ui::widget::ImageNode};
use gdtf_battle_presenter::TileRoles;
use gdtf_battle_sim::{
    Cell,
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    level::{GridHeight, GridLevels, GridSize, GridWidth, UuidThemeRegistry},
    terrain::{
        def::{
            TerrainDef, TerrainDefRegistry, TerrainDisplayName, TerrainPresenterKind,
            TerrainSimKind, TerrainUuid,
        },
        piece::TerrainGraphicKey,
    },
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
    settle(&mut app);
    app
}

/// Advance a handful of frames so the deferred re-parent + the `sync_canvas` build + the ghost
/// spawn have all applied.
fn settle(app: &mut App) {
    for _ in 0..8 {
        app.update();
    }
}

/// Sets `entity`'s [`Interaction`] to `state` and runs ONLY the `Update` schedule so the
/// `Changed<Interaction>` driver fires on the edge.
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

/// The graphic role key of a presenter kind (every variant carries it).
fn graphic_role(def: &TerrainDef) -> &str {
    match &def.presenter_kind {
        TerrainPresenterKind::Wall { graphic_name }
        | TerrainPresenterKind::Cover { graphic_name }
        | TerrainPresenterKind::Slab { graphic_name, .. } => graphic_name,
    }
}

/// The active theme's resolved default-floor atlas index (the fill every unpainted cell shows).
fn default_floor_index(app: &App) -> Option<usize> {
    let session = app.world().get_resource::<MapEditorSession>()?;
    let themes = app.world().get_resource::<UuidThemeRegistry>()?;
    let terrain = app.world().get_resource::<TerrainDefRegistry>()?;
    let roles = app.world().get_resource::<TileRoles>()?;
    let floor = session
        .default_floor()
        .or_else(|| themes.default_floor(&session.theme()))?;
    let def = terrain.def(&floor)?;
    roles.index_for_key(graphic_role(def)).map(|i| *i)
}

/// Pick a terrain in the active theme's palette whose resolved atlas index DIFFERS from the
/// default-floor's. Returns the terrain key + its resolved index.
fn distinct_paint_tile(app: &App) -> Option<(TerrainUuid, usize)> {
    let session = app.world().get_resource::<MapEditorSession>()?;
    let themes = app.world().get_resource::<UuidThemeRegistry>()?;
    let terrain = app.world().get_resource::<TerrainDefRegistry>()?;
    let roles = app.world().get_resource::<TileRoles>()?;
    let default_index = default_floor_index(app)?;
    let palette = themes.terrain(&session.theme())?;
    palette.iter().find_map(|key| {
        let def = terrain.def(key)?;
        let index = *roles.index_for_key(graphic_role(def))?;
        (index != default_index).then_some((*key, index))
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

/// The [`TerrainUuid`] of a SLAB terrain in the active theme's palette, if any.
fn slab_tile_key(app: &App) -> Option<TerrainUuid> {
    let session = app.world().get_resource::<MapEditorSession>()?;
    let themes = app.world().get_resource::<UuidThemeRegistry>()?;
    let terrain = app.world().get_resource::<TerrainDefRegistry>()?;
    let palette = themes.terrain(&session.theme())?;
    palette.iter().find_map(|key| {
        terrain
            .def(key)
            .filter(|def| matches!(def.sim_kind, TerrainSimKind::Slab { .. }))
            .map(|_| *key)
    })
}

/// The single [`CanvasGhost`] entity, if it exists.
fn cell_ghost_entity(app: &mut App) -> Option<Entity> {
    let world = app.world_mut();
    let mut q = world.query_filtered::<Entity, With<CanvasGhost>>();
    q.iter(world).next()
}

/// The [`CanvasGhost`]'s [`ImageNode`] color, if the single ghost exists.
fn ghost_color(app: &mut App) -> Option<Color> {
    let world = app.world_mut();
    let mut q = world.query_filtered::<&ImageNode, With<CanvasGhost>>();
    q.iter(world).next().map(|node| node.color)
}

/// Whether `color` reads as RED — its red channel strictly greater than BOTH green and blue.
fn is_red_dominant(color: Color) -> bool {
    let rgba = color.to_srgba();
    rgba.red > rgba.green && rgba.red > rgba.blue
}

/// The synthetic ladder terrain key the test seeds (shipped content authors no ladder terrain).
const LADDER_KEY: TerrainUuid =
    TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0EDD_0000_0000_0001));

/// Seed a LADDER terrain def (display name "Steel Ladder", recognised by the GTW-430 ladder-naming
/// convention) into the live [`TerrainDefRegistry`] AND paint it into the [`EditorMap`] at `cell`.
/// The shipped content authors no ladder terrain, so the test seeds one directly so the GTW-430
/// illegal slab-over-ladder case is reachable. Returns whether the model seed landed.
fn seed_ladder(app: &mut App, cell: Cell) -> bool {
    let ladder = TerrainDef {
        key:            LADDER_KEY,
        display_name:   TerrainDisplayName::new("Steel Ladder".to_owned()),
        sim_kind:       TerrainSimKind::Cover {
            hp:               CoverHp::new(10),
            armor_protection: ArmorProtection::new(0),
            armor_hardness:   ArmorHardness::new(0),
            height_band:      HeightBand::Low,
        },
        presenter_kind: TerrainPresenterKind::Cover {
            graphic_name: TerrainGraphicKey::new("ladder".to_owned()),
        },
        tags:           Vec::new(),
    };
    if let Some(mut registry) = app.world_mut().get_resource_mut::<TerrainDefRegistry>() {
        registry.insert(LADDER_KEY, ladder);
    }
    let Some(size) = grid_size(app) else {
        return false;
    };
    let Some(mut map) = app.world_mut().get_resource_mut::<EditorMap>() else {
        return false;
    };
    map.paint(cell, LADDER_KEY, size)
}

/// T1 (C2/C3): pressing a canvas cell with a SELECTED tile records that cell in the [`EditorMap`]
/// model AND redraws the cell's [`ImageNode`] atlas index to the selected terrain's resolved index.
#[test]
fn pressing_a_cell_paints_the_model_and_redraws_the_sprite() {
    let mut app = editor_in_editing();

    let pick = distinct_paint_tile(&app);
    assert!(
        pick.is_some(),
        "the active theme must have a terrain distinct from its default floor to paint with (C2)",
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
        session.select_tile(paint_key);
    }

    let target = Cell::new(2, 3);
    let entity = cell_entity_at(&mut app, target);
    assert!(
        entity.is_some(),
        "the canvas must have a CanvasCell at (2, 3) to paint (the grid is at least 16×16)",
    );

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

    let map = app.world().get_resource::<EditorMap>();
    if let Some(map) = map {
        assert_eq!(
            map.painted_count(),
            1,
            "exactly one cell must be painted in the model after one press (C2)",
        );
        assert_eq!(
            map.tile_at(target),
            Some(paint_key),
            "the model must hold the painted cell keyed by its Cell, with the selected tile (C2)",
        );
    }
    assert_eq!(
        cell_index_at(&mut app, target),
        Some(paint_index),
        "the painted cell's sprite must redraw to the selected terrain's resolved index (C2/C3)",
    );
}

/// T2 (C3 clamp): the model only ever holds IN-BOUNDS cells.
#[test]
fn paint_is_clamped_to_the_drawable_extent() {
    let mut app = editor_in_editing();

    let size = grid_size(&app);
    assert!(size.is_some(), "the session must hold a grid size");
    let Some(size) = size else {
        return;
    };

    let mut model = EditorMap::new();
    let in_bounds = Cell::new(0, 0);
    let key = TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x1234));
    let recorded_in = model.paint(in_bounds, key, size);
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
        let recorded_out = model.paint(out, key, size);
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

    // The live press path also only ever records in-bounds cells.
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

/// T3 (C1): the hover ghost appears at the hovered cell when a tile is selected, and is HIDDEN
/// when no cell is hovered or no tile is selected.
#[test]
fn hover_ghost_follows_the_hovered_cell_and_hides_otherwise() {
    let mut app = editor_in_editing();

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

    set_interaction(&mut app, cell, Interaction::None);
    assert_eq!(
        app.world().get::<Visibility>(ghost),
        Some(&Visibility::Hidden),
        "the ghost must hide when no cell is hovered (C1)",
    );

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

/// T4 (GTW-430 C2/C3): hovering an ILLEGAL placement tints the hover-ghost preview RED — through
/// the LIVE [`follow_hover_ghost`] + the SHARED `evaluate_placement`.
#[test]
fn illegal_hover_tints_the_ghost_red() {
    let mut app = editor_in_editing();

    let slab = slab_tile_key(&app);
    assert!(
        slab.is_some(),
        "the active theme must have a slab tile to drive the illegal slab-over-ladder hover (C2)",
    );
    let Some(slab) = slab else {
        return;
    };

    // Seed a ladder def + a ladder cell on the ground plane (shipped content ships no ladder).
    let ladder_cell = Cell::new(5, 5);
    assert!(
        seed_ladder(&mut app, ladder_cell),
        "the ladder seed must land in-bounds on the model",
    );

    if let Some(mut session) = app.world_mut().get_resource_mut::<MapEditorSession>() {
        session.select_tile(slab);
    }
    let cell = cell_entity_at(&mut app, ladder_cell);
    assert!(
        cell.is_some(),
        "the canvas must have a cell at the seeded ladder coordinate to hover",
    );
    let Some(cell) = cell else {
        return;
    };
    set_interaction(&mut app, cell, Interaction::Hovered);

    if let Some(ghost) = cell_ghost_entity(&mut app) {
        assert_eq!(
            app.world().get::<Visibility>(ghost),
            Some(&Visibility::Visible),
            "the ghost must be visible while previewing the (illegal) placement (C2)",
        );
    }
    let illegal = ghost_color(&mut app);
    assert!(illegal.is_some(), "the ghost must exist to read its tint");
    if let Some(illegal) = illegal {
        assert!(
            is_red_dominant(illegal),
            "the ghost over an illegal slab-over-ladder placement must tint RED — red channel \
             strictly dominant (GTW-430 C2); got {illegal:?}",
        );

        let clear_cell = Cell::new(8, 8);
        if let Some(clear) = cell_entity_at(&mut app, clear_cell) {
            set_interaction(&mut app, clear, Interaction::Hovered);
        }
        let legal = ghost_color(&mut app);
        if let Some(legal) = legal {
            assert_ne!(
                illegal, legal,
                "the illegal tint must DIFFER from the legal preview — the verdict flipped it (C2)",
            );
            assert!(
                !is_red_dominant(legal),
                "the legal preview must NOT be red-dominant (it is the white preview); got {legal:?}",
            );
        }
    }
}

/// T5 (GTW-430 C2): hovering a LEGAL placement tints the hover-ghost preview the NORMAL (non-red)
/// preview — the symmetric counterpart of T4.
#[test]
fn legal_hover_tints_the_ghost_the_normal_preview() {
    let mut app = editor_in_editing();

    let slab = slab_tile_key(&app);
    assert!(slab.is_some(), "the active theme must have a slab tile");
    let Some(slab) = slab else {
        return;
    };
    if let Some(mut session) = app.world_mut().get_resource_mut::<MapEditorSession>() {
        session.select_tile(slab);
    }

    let target = Cell::new(7, 7);
    let cell = cell_entity_at(&mut app, target);
    assert!(
        cell.is_some(),
        "the canvas must have an empty cell to hover"
    );
    let Some(cell) = cell else {
        return;
    };
    set_interaction(&mut app, cell, Interaction::Hovered);

    if let Some(ghost) = cell_ghost_entity(&mut app) {
        assert_eq!(
            app.world().get::<Visibility>(ghost),
            Some(&Visibility::Visible),
            "the ghost must be visible while previewing the legal placement (C2)",
        );
    }
    let legal = ghost_color(&mut app);
    assert!(legal.is_some(), "the ghost must exist to read its tint");
    if let Some(legal) = legal {
        assert!(
            !is_red_dominant(legal),
            "the ghost over a LEGAL placement must NOT be red-dominant (it is the normal preview \
             tint, GTW-430 C2); got {legal:?}",
        );
    }
}
