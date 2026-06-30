//! Headless integration tests for the GTW-500 PREFAB-canvas UX: level navigation (C1), centred
//! prefab (C2), and mouse-wheel zoom (C3).
//!
//! Drives the REAL [`MapEditorPlugin`] on the no-renderer `DefaultPlugins` UI harness (mirrors
//! `tests/canvas.rs`), so the editor's actual `Load` pass + `Editing` scene build the canvas + wire
//! the new systems — not a copy. Each behavioural clause has an assertion that would FAIL if the
//! clause were reverted; tunable constants (zoom min/max/step) are never pinned beyond what the
//! clause logically requires.
//!
//! - T1 (C1): a level-up key step advances [`CurrentEditLevel`]; it CLAMPS at both ends.
//! - T2 (C1): driving the REAL `paint_cell` system (a cell-press edge) on a NON-ZERO level writes
//!   that storey's [`CellLevel`] in the `EditorMap`, leaving the ground plane untouched.
//! - T2r (C1): the RENDER follows the level — distinct tiles painted on `L0` vs `L1` (in the model)
//!   make the rendered [`CanvasCell`] [`ImageNode`] atlas index change when the level steps.
//! - T3 (C3): a [`MouseWheel`] over the canvas (cursor-gated) grows [`CanvasZoom`] AND the
//!   [`CanvasCell`] `Node` width/height; the `0` reset returns it to `1.0`.
//! - T3b (C3): a wheel with the cursor NOT over the canvas does NOT zoom (the gate holds).
//! - T3c (C3): zoom is CURSOR-ANCHORED — zooming in with the cursor parked off the viewport
//!   top-left re-anchors the canvas [`ScrollPosition`] (it grows), keeping the content point under
//!   the cursor fixed; deleting the cursor-anchor re-scroll leaves it at `0`.
//! - T4 (C2): a SMALL grid (smaller than the viewport) centres — the [`CanvasRoot`]
//!   [`UiTransform`] translation is a non-zero offset; a LARGE grid overflows — the translation is
//!   zero on the overflowing axis.
//!
//! Panic/expect-free per the workspace lints.

use bevy::{
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
        mouse::{MouseScrollUnit, MouseWheel},
    },
    prelude::*,
    ui::{
        ComputedNode, Node, ScrollPosition, UiGlobalTransform, UiTransform, Val, widget::ImageNode,
    },
    window::{PrimaryWindow, Window, WindowResolution},
};
use gdtf_battle_presenter::TileRoles;
use gdtf_battle_sim::{
    Cell,
    metric::{CellLevel, Level},
    terrain::def::{TerrainDef, TerrainDefRegistry, TerrainPresenterKind, TerrainUuid},
};
use gdtf_content_editor::{
    CanvasCell, CanvasRoot, CanvasZoom, CurrentEditLevel, EditorMap, EditorState, EditorTileClass,
    GridSpanInput, LevelNavButton, MapEditorPlugin, MapEditorSession, SizeFieldAxis, classify,
};
use gdtf_test_utils::{GdtfUiTestAppBuilder, advance_until};
use gdtf_ui::{CommittedNumericValue, NumericFieldCommitted, ScrollListArea, UiPlugin};

/// A generous frame cap: async asset loads under parallel `cargo` contention take a
/// non-deterministic number of frames, so this is a SAFETY NET — we poll the
/// `EditorState::Editing` SIGNAL, not a fixed count.
const MAX_UPDATES: u32 = 10_000;

/// The headless UI harness lays UI out against a target's size; with NO window that size is zero,
/// so percent/flex `ComputedNode`s collapse to 0 and the centring / zoom-cursor geometry is
/// untestable. A real-resolution [`PrimaryWindow`] gives the layout a viewport to resolve against.
const TEST_WINDOW: UVec2 = UVec2::new(1280, 720);

/// Builds the real editor app on the no-renderer `DefaultPlugins` UI harness — with a sized
/// [`PrimaryWindow`] so the UI lays out against a real viewport (the `ComputedNode` sizes the
/// centring / zoom-cursor tests read) — and advances it to [`EditorState::Editing`] with the canvas
/// built across the deferred-parent frames.
fn editor_in_editing() -> App {
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    app.add_plugins(UiPlugin);
    app.add_plugins(MapEditorPlugin);
    app.world_mut().spawn((
        Window {
            resolution: WindowResolution::new(TEST_WINDOW.x, TEST_WINDOW.y),
            ..default()
        },
        PrimaryWindow,
    ));

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

/// Advance a handful of frames so the deferred scroll-list re-parent + the seed + the
/// `sync_canvas` rebuild + its deferred grid re-parent + the zoom re-layout + the centring have
/// all applied + the layout has settled (a `ComputedNode` read needs the layout to converge).
fn settle(app: &mut App) {
    for _ in 0..12 {
        app.update();
    }
}

/// The session's current `grid_size`, if present.
fn current_level(app: &App) -> Option<CurrentEditLevel> {
    app.world().get_resource::<CurrentEditLevel>().copied()
}

/// The current `CanvasZoom`, if present.
fn current_zoom(app: &App) -> Option<CanvasZoom> {
    app.world().get_resource::<CanvasZoom>().copied()
}

/// A presenter kind's graphic-role key (every variant carries one) — the resolution path the canvas
/// uses (the `canvas.rs` test precedent), so a tile's rendered index is value-agnostically derived.
fn graphic_role(def: &TerrainDef) -> &str {
    match &def.presenter_kind {
        TerrainPresenterKind::Wall { graphic_name }
        | TerrainPresenterKind::Cover { graphic_name }
        | TerrainPresenterKind::Slab { graphic_name, .. } => graphic_name,
    }
}

/// The atlas index a painted `tile` fills with — resolved THE WAY THE PRESENTER / CANVAS DO (the
/// def's graphic role against [`TileRoles`]), so the test pins the rendered index to the same source
/// the system reads, never a hard-coded magnitude.
fn paint_index(app: &App, tile: TerrainUuid) -> Option<usize> {
    let registry = app.world().get_resource::<TerrainDefRegistry>()?;
    let roles = app.world().get_resource::<TileRoles>()?;
    let def = registry.def(&tile)?;
    roles.index_for_key(graphic_role(def)).map(|i| *i)
}

/// Up to `want` distinct terrain UUIDs that (a) classify as [`EditorTileClass::Other`] — so a paint
/// is unconditionally legal (no slab-seals-ladder rule) — and (b) resolve to DISTINCT atlas indices,
/// so a render swap between them is observable. Pulled from the live registry so the test stays
/// value-agnostic.
fn distinct_paintable_tiles(app: &App, want: usize) -> Vec<TerrainUuid> {
    let theme = app
        .world()
        .get_resource::<MapEditorSession>()
        .map(MapEditorSession::theme);
    let registry = app.world().get_resource::<TerrainDefRegistry>();
    let (Some(theme), Some(registry)) = (theme, registry) else {
        return Vec::new();
    };
    let mut seen_indices: Vec<usize> = Vec::new();
    let mut out: Vec<TerrainUuid> = Vec::new();
    for (key, _) in registry.defs() {
        if classify(registry, theme, key) != EditorTileClass::Other {
            continue;
        }
        let Some(index) = paint_index(app, *key) else {
            continue;
        };
        if seen_indices.contains(&index) {
            continue;
        }
        seen_indices.push(index);
        out.push(*key);
        if out.len() == want {
            break;
        }
    }
    out
}

/// The rendered atlas index of the `CanvasCell` at ground-plane coordinate `cell`, if that cell
/// exists and carries an `ImageNode` atlas — what the SLICE actually draws (the render half of C1).
fn rendered_index_at(app: &mut App, cell: Cell) -> Option<usize> {
    let world = app.world_mut();
    let mut q = world.query::<(&CanvasCell, &ImageNode)>();
    q.iter(world).find_map(|(canvas_cell, node)| {
        (canvas_cell.cell() == cell)
            .then(|| node.texture_atlas.as_ref().map(|atlas| atlas.index))
            .flatten()
    })
}

/// Press a `CanvasCell` at ground-plane coordinate `cell` — set its `Interaction` to `Pressed` and
/// run ONLY the `Update` schedule so the REAL `paint_cell` system (which reads
/// `Changed<Interaction> == Pressed` on a `CanvasCell`) commits a paint through its
/// `apply_placement` -> `EditorMap::paint_at` path on the CURRENT level. Running `Update` alone (the
/// `canvas_paint.rs` `set_interaction` precedent), NOT a full `app.update()`, is required: a full
/// frame's `PreUpdate` `ui_focus_system` would reset the `Interaction` to `None` (no real cursor is
/// over the cell) before `paint_cell` reads it. This drives the SAME system under test (bevy-traps
/// #6), not a copy.
fn press_cell(app: &mut App, cell: Cell) {
    let target = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &CanvasCell)>();
        q.iter(world)
            .find_map(|(entity, canvas_cell)| (canvas_cell.cell() == cell).then_some(entity))
    };
    if let Some(target) = target
        && let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(target)
    {
        *interaction = Interaction::Pressed;
    }
    app.world_mut().run_schedule(Update);
}

/// Tap a single keyboard key as a just-pressed edge, then settle.
///
/// Drives the REAL `bevy_input` `keyboard_input_system` by writing a [`KeyboardInput`] message:
/// under `DefaultPlugins` that system clears + repopulates `ButtonInput<KeyCode>` in `PreUpdate`
/// each frame, so directly mutating the resource before `update()` would be wiped before the
/// editor's `Update` reads it — the message path sets `just_pressed` correctly for the frame.
fn tap_key(app: &mut App, key: KeyCode) {
    app.world_mut().write_message(KeyboardInput {
        key_code:    key,
        logical_key: Key::Unidentified(bevy::input::keyboard::NativeKey::Unidentified),
        state:       ButtonState::Pressed,
        text:        None,
        repeat:      false,
        window:      Entity::PLACEHOLDER,
    });
    app.update();
    // Release on the next frame so a later tap is a fresh just-pressed.
    app.world_mut().write_message(KeyboardInput {
        key_code:    key,
        logical_key: Key::Unidentified(bevy::input::keyboard::NativeKey::Unidentified),
        state:       ButtonState::Released,
        text:        None,
        repeat:      false,
        window:      Entity::PLACEHOLDER,
    });
    settle(app);
}

/// The first `CanvasCell`'s `Node` width as a logical-px value, if it is a `Px` value.
fn a_cell_width_px(app: &mut App) -> Option<f32> {
    let world = app.world_mut();
    let mut q = world.query_filtered::<&Node, With<CanvasCell>>();
    q.iter(world).next().and_then(|node| match node.width {
        Val::Px(px) => Some(px),
        _ => None,
    })
}

/// T1 (C1): a level-UP key step advances [`CurrentEditLevel`]; repeated steps CLAMP at the
/// ceiling (`levels-1`) and a DOWN step from the ground clamps at `0`.
///
/// Pin-discriminating: reverting the level-nav write leaves the level at `0` (the up step fails);
/// dropping the clamp lets the level run past `levels-1` (the ceiling assert fails) or below `0`
/// (the floor assert fails).
#[test]
fn level_keys_step_and_clamp() {
    let mut app = editor_in_editing();

    let start = current_level(&app);
    assert_eq!(
        start.map(|l| *l.level()),
        Some(0),
        "the editor opens on the ground storey (C1)",
    );
    let levels = app
        .world()
        .get_resource::<MapEditorSession>()
        .map(|s| *s.grid_size().levels());
    assert!(levels.is_some(), "the session must hold a grid size");
    let max_storey = levels.map_or(0, |n| n.saturating_sub(1));
    assert!(
        max_storey > 0,
        "the default grid must have more than one storey for the step to be observable (C1)",
    );

    // One step UP advances the level.
    tap_key(&mut app, KeyCode::BracketRight);
    assert_eq!(
        current_level(&app).map(|l| *l.level()),
        Some(1),
        "a level-up key (']') must advance CurrentEditLevel (C1)",
    );

    // Many steps UP saturate at the ceiling (levels-1), never past it.
    for _ in 0..(usize::from(max_storey) + 4) {
        tap_key(&mut app, KeyCode::BracketRight);
    }
    assert_eq!(
        current_level(&app).map(|l| *l.level()),
        Some(max_storey),
        "stepping up past the top must CLAMP at levels-1 (C1)",
    );

    // Many steps DOWN saturate at the ground (0), never below it.
    for _ in 0..(usize::from(max_storey) + 4) {
        tap_key(&mut app, KeyCode::BracketLeft);
    }
    assert_eq!(
        current_level(&app).map(|l| *l.level()),
        Some(0),
        "stepping down past the ground must CLAMP at 0 (C1)",
    );
}

/// T1b (C1): the chrome up/down BUTTONS exist (a [`LevelNavButton`] per direction), so the level
/// can be navigated by mouse too — the keyboard is not the only path (C1).
///
/// Pin-discriminating: dropping the chrome buttons drops the markers, failing the count.
#[test]
fn level_nav_buttons_exist() {
    let mut app = editor_in_editing();
    let buttons = {
        let world = app.world_mut();
        let mut q = world.query::<&LevelNavButton>();
        q.iter(world).count()
    };
    assert_eq!(
        buttons, 2,
        "the canvas chrome must carry an up AND a down level-nav button (C1)",
    );
}

/// T2 (C1): driving the REAL `paint_cell` system (a `CanvasCell` press) on a NON-ZERO level writes
/// that storey's `CellLevel` in the `EditorMap`, and the ground plane stays unpainted — proving the
/// SYSTEM's paint flow consumes `CurrentEditLevel`, not a hardcoded ground plane.
///
/// Pin-discriminating ON THE SYSTEM: reverting `paint_cell`'s `level.level()` to a hardcoded
/// `GROUND_LEVEL` would commit the paint at `L0`, so the `L1` read would be `None` (fail) and the
/// `L0` read non-`None` (fail) — both asserts flip. Unlike a direct `map.paint_at`, this drives the
/// system under test (verification.md Rule 2): the press edge → `paint_cell` → `apply_placement` →
/// `EditorMap::paint_at(slot)` where `slot` is built from `CurrentEditLevel` INSIDE the system.
#[test]
fn paint_targets_the_current_level() {
    let mut app = editor_in_editing();

    // Select a paintable tile (classifies as `Other`, so the placement is unconditionally legal) and
    // step up to L1, so the only thing that decides which storey records the paint is the system's
    // read of `CurrentEditLevel`.
    let tiles = distinct_paintable_tiles(&app, 1);
    let Some(&tile) = tiles.first() else {
        // No registered terrain resolves to a sprite — the editor's content is broken, not this
        // test's premise; skip rather than false-fail.
        return;
    };
    if let Some(mut session) = app.world_mut().get_resource_mut::<MapEditorSession>() {
        session.select_tile(tile);
    }
    tap_key(&mut app, KeyCode::BracketRight);
    let level = current_level(&app).map(CurrentEditLevel::level);
    assert_eq!(level.map(|l| *l), Some(1), "must be on L1 to paint there");

    // Press a specific cell — the REAL `paint_cell` system commits the paint this frame.
    let cell = Cell::new(2, 3);
    press_cell(&mut app, cell);

    let Some(level) = level else {
        return;
    };
    let slot = CellLevel::new(cell, level);
    let ground = CellLevel::new(cell, Level::new(0));
    let map = app.world().get_resource::<EditorMap>();
    assert_eq!(
        map.and_then(|m| m.tile_at_level(slot)),
        Some(tile),
        "the SYSTEM's paint must record on the CURRENT storey (L1), not the ground plane (C1)",
    );
    assert!(
        map.and_then(|m| m.tile_at_level(ground)).is_none(),
        "the same x/y on the ground plane must stay unpainted — the system wrote level-scoped (C1)",
    );
}

/// T2r (C1, the RENDER half): the rendered slice follows `CurrentEditLevel`. With DISTINCT tiles
/// painted on `L0` and `L1` (in the model), the rendered `CanvasCell` `ImageNode` atlas index at the
/// painted coordinate shows the `L0` tile while on the ground and the `L1` tile after stepping up.
///
/// Pin-discriminating ON `sync_canvas`: reverting it to hard-code `Level::new(0)` (ignoring
/// `CurrentEditLevel`) would keep the rendered index at the `L0` tile after the step — the
/// after-step assert (rendered index == the `L1` tile's index) flips. The indices are RESOLVED via
/// the presenter role table, never pinned.
#[test]
fn render_follows_the_current_level() {
    let mut app = editor_in_editing();

    // Two distinct, resolvable, legal tiles so the L0 vs L1 slices render to different atlas indices.
    let tiles = distinct_paintable_tiles(&app, 2);
    let (Some(&ground_tile), Some(&upper_tile)) = (tiles.first(), tiles.get(1)) else {
        // The content lacks two distinctly-rendering terrains — skip rather than false-fail.
        return;
    };
    let ground_index = paint_index(&app, ground_tile);
    let upper_index = paint_index(&app, upper_tile);
    assert!(
        ground_index.is_some() && upper_index.is_some() && ground_index != upper_index,
        "the two slice tiles must resolve to distinct atlas indices for the swap to be observable",
    );

    let size = app
        .world()
        .get_resource::<MapEditorSession>()
        .map(MapEditorSession::grid_size);
    let Some(size) = size else {
        return;
    };
    let cell = Cell::new(1, 1);
    // Seed the MODEL directly (this is test SETUP of map state, not the unit under test — the unit
    // under test is `sync_canvas`'s level-aware READ of that model). Both storeys of `cell` are now
    // painted with DISTINCT tiles.
    if let Some(mut map) = app.world_mut().get_resource_mut::<EditorMap>() {
        assert!(map.paint_at(CellLevel::new(cell, Level::new(0)), ground_tile, size));
        assert!(map.paint_at(CellLevel::new(cell, Level::new(1)), upper_tile, size));
    }

    // Step UP to L1 — this CHANGES `CurrentEditLevel`, so `sync_canvas` rebuilds reading the L1 slice
    // (a model write alone does not trigger a rebuild; the level step does). The rebuilt cell must
    // render the L1-painted tile.
    tap_key(&mut app, KeyCode::BracketRight);
    assert_eq!(
        current_level(&app).map(|l| *l.level()),
        Some(1),
        "must be on L1 after the step",
    );
    assert_eq!(
        rendered_index_at(&mut app, cell),
        upper_index,
        "on L1 the rendered cell must show the L1-painted tile's atlas index — the render follows \
         CurrentEditLevel, not a hardcoded ground plane (C1 render)",
    );

    // Step back DOWN to L0 — `sync_canvas` rebuilds the L0 slice, which must now render the L0 tile,
    // NOT the L1 one. Reverting `sync_canvas` to hard-code `Level::new(0)` would render the L0 tile
    // on BOTH storeys, flipping the L1 assert above.
    tap_key(&mut app, KeyCode::BracketLeft);
    assert_eq!(
        current_level(&app).map(|l| *l.level()),
        Some(0),
        "must be back on L0 after the down step",
    );
    assert_eq!(
        rendered_index_at(&mut app, cell),
        ground_index,
        "back on L0 the rendered cell must show the L0-painted tile's atlas index (C1 render)",
    );
}

/// The CANVAS's scroll-area entity — the [`ScrollListArea`] ancestor of the [`CanvasRoot`] (the
/// editor has THREE scroll areas: palette / canvas / right panel; only the canvas's hosts the grid).
/// Walks up the `ChildOf` chain from the root to the first `ScrollListArea`, so the cursor-gate
/// tests place the cursor over the CANVAS viewport, not an arbitrary one.
fn canvas_scroll_area(app: &mut App) -> Option<Entity> {
    let world = app.world_mut();
    let root = {
        let mut q = world.query_filtered::<Entity, With<CanvasRoot>>();
        q.iter(world).next()
    }?;
    let area_set: Vec<Entity> = {
        let mut areas = world.query_filtered::<Entity, With<ScrollListArea>>();
        areas.iter(world).collect()
    };
    let mut parents = world.query::<&ChildOf>();
    let mut cursor = Some(root);
    for _ in 0..64 {
        let current = cursor?;
        if area_set.contains(&current) {
            return Some(current);
        }
        cursor = parents.get(world, current).ok().map(ChildOf::parent);
    }
    None
}

/// The CANVAS scroll area's computed size + global-transform centre (physical px), if present.
fn area_geometry(app: &mut App) -> Option<(Vec2, Vec2)> {
    let area = canvas_scroll_area(app)?;
    let world = app.world_mut();
    let node = world.get::<ComputedNode>(area)?;
    let size = node.size();
    let transform = world.get::<UiGlobalTransform>(area)?;
    Some((size, transform.translation))
}

/// The canvas scroll area's [`ScrollPosition`] (logical px), if present — what the cursor-anchored
/// zoom re-anchors. Read off the CANVAS's own scroll area (the one the real gate uses).
fn canvas_scroll_position(app: &mut App) -> Option<Vec2> {
    let area = canvas_scroll_area(app)?;
    app.world().get::<ScrollPosition>(area).map(|p| p.0)
}

/// Whether the primary window's physical cursor is currently over the CANVAS scroll area, computed
/// the SAME way the real `read_zoom_wheel` gate does ([`ComputedNode::contains_point`] on the
/// canvas's own scroll area). Used by the off-canvas gate test to assert the cursor is genuinely
/// outside before wheeling, so the no-zoom assertion runs unconditionally.
fn cursor_over_canvas(app: &mut App) -> bool {
    let Some(area) = canvas_scroll_area(app) else {
        return false;
    };
    let world = app.world_mut();
    let cursor = world
        .query_filtered::<&Window, With<PrimaryWindow>>()
        .iter(world)
        .next()
        .and_then(bevy::window::Window::physical_cursor_position);
    let Some(cursor) = cursor else {
        return false;
    };
    let node = world.get::<ComputedNode>(area);
    let transform = world.get::<UiGlobalTransform>(area);
    matches!((node, transform), (Some(n), Some(t)) if n.contains_point(*t, cursor))
}

/// Place the (already-spawned) primary window's PHYSICAL cursor at `physical`, then settle so the
/// cursor-gated systems observe it.
fn set_cursor(app: &mut App, physical: Vec2) {
    let mut windows = app
        .world_mut()
        .query_filtered::<&mut Window, With<PrimaryWindow>>();
    for mut window in windows.iter_mut(app.world_mut()) {
        window.set_physical_cursor_position(Some(physical.as_dvec2()));
    }
    settle(app);
}

/// Write a single line-unit `MouseWheel` message (positive y = zoom in) and settle.
fn wheel(app: &mut App, lines: f32) {
    app.world_mut().write_message(MouseWheel {
        unit:   MouseScrollUnit::Line,
        x:      0.0,
        y:      lines,
        window: Entity::PLACEHOLDER,
        phase:  bevy::input::touch::TouchPhase::Moved,
    });
    settle(app);
}

/// T3 (C3): a [`MouseWheel`] over the canvas (cursor-gated) grows [`CanvasZoom`] AND each
/// [`CanvasCell`] `Node` edge; the `0` reset returns the zoom to `1.0`.
///
/// Pin-discriminating: reverting the wheel read leaves the zoom at `1.0` (the grow assert fails);
/// reverting the cell re-layout leaves the `Node` width unchanged (the width-grew assert fails);
/// reverting the reset leaves the zoom > `1.0` after `0` (the reset assert fails). The asserts use
/// `>`/`==1.0`, never a pinned zoom magnitude.
#[test]
fn wheel_over_canvas_zooms_and_resizes_cells() {
    let mut app = editor_in_editing();

    let start_zoom = current_zoom(&app).map(|z| *z);
    assert_eq!(start_zoom, Some(1.0), "the editor opens unzoomed (C3)");
    let start_width = a_cell_width_px(&mut app);
    assert!(start_width.is_some(), "a cell must have a Px width to grow");

    // Place the cursor at the centre of the scroll viewport so the cursor-over-canvas gate passes.
    let geometry = area_geometry(&mut app);
    assert!(
        geometry.is_some(),
        "the scroll area must have a computed layout"
    );
    let Some((size, centre)) = geometry else {
        return;
    };
    assert!(
        size.x > 0.0 && size.y > 0.0,
        "the scroll viewport must have a non-zero computed size to host the cursor",
    );
    set_cursor(&mut app, centre);

    // Zoom IN: three notches up.
    wheel(&mut app, 3.0);

    let zoomed = current_zoom(&app).map(|z| *z);
    assert!(
        zoomed.is_some_and(|z| z > 1.0),
        "a wheel turn UP over the canvas must GROW the zoom (C3), got {zoomed:?}",
    );
    let grown_width = a_cell_width_px(&mut app);
    assert!(
        matches!((start_width, grown_width), (Some(s), Some(g)) if g > s),
        "zooming must re-lay-out the cell Node width LARGER (C3): {start_width:?} -> {grown_width:?}",
    );

    // The `0` reset returns the zoom to 1.0 and the cells to their base edge.
    tap_key(&mut app, KeyCode::Digit0);
    assert_eq!(
        current_zoom(&app).map(|z| *z),
        Some(1.0),
        "the '0' hotkey must RESET the zoom to 1.0 (C3)",
    );
    let reset_width = a_cell_width_px(&mut app);
    assert_eq!(
        reset_width, start_width,
        "resetting the zoom must restore the base cell edge (C3)",
    );
}

/// T3b (C3): a [`MouseWheel`] with the cursor NOT over the canvas does NOT zoom — the gate holds.
///
/// Pin-discriminating UNCONDITIONALLY: the cursor is parked FAR outside the `1280×720` window (a
/// coordinate no laid-out node can contain), and the test ASSERTS it is not over the canvas (via the
/// same [`ComputedNode::contains_point`] check the real gate uses) BEFORE wheeling — so the body is
/// never vacuous. Dropping the cursor-over-canvas gate would let this off-canvas wheel zoom, flipping
/// the "unchanged" assert; the gate is therefore robustly failing-on-revert, not conditionally.
#[test]
fn wheel_off_canvas_does_not_zoom() {
    let mut app = editor_in_editing();
    assert_eq!(current_zoom(&app).map(|z| *z), Some(1.0));

    // A cursor FAR outside the window (and thus outside every laid-out viewport) — a deterministic
    // off-canvas point well past the `1280×720` resolution, not the layout-dependent (2,2) corner.
    let far_outside = Vec2::splat(100_000.0);
    set_cursor(&mut app, far_outside);
    // GUARANTEE the premise: the cursor is genuinely NOT over the canvas, so the no-zoom assert below
    // is unconditional (never a vacuous pass). Computed exactly as the real `read_zoom_wheel` gate.
    assert!(
        !cursor_over_canvas(&mut app),
        "the test's off-canvas cursor must resolve OUTSIDE the canvas scroll area so the no-zoom \
         assertion is unconditional",
    );

    wheel(&mut app, 3.0);
    assert_eq!(
        current_zoom(&app).map(|z| *z),
        Some(1.0),
        "a wheel turn with the cursor NOT over the canvas must NOT zoom (C3 gate)",
    );
}

/// T3c (C3): zoom is CURSOR-ANCHORED — zooming IN with the cursor parked OFF the viewport top-left
/// re-anchors the canvas [`ScrollPosition`] so the content point under the cursor stays put. On the
/// default (overflowing) grid the re-anchor produces a POSITIVE scroll offset where there was none.
///
/// Pin-discriminating ON THE CURSOR-ANCHOR BLOCK: reverting `apply_canvas_zoom`'s
/// `new_scroll = (scroll + cursor) * ratio - cursor` re-scroll (deleting the whole cursor-anchored
/// `ScrollPosition` write) leaves the scroll at `0` — the headless harness has no picking backend to
/// emit `Pointer<Scroll>`, so nothing else moves it — so the "scroll grew" assert flips. The default
/// grid (60×60 = 1440 px) overflows the centre viewport, so `max > 0` and the clamp does not erase
/// the offset. The assert is `> 0` / `> before`, never a pinned scroll magnitude.
#[test]
fn zoom_is_cursor_anchored() {
    let mut app = editor_in_editing();
    assert_eq!(current_zoom(&app).map(|z| *z), Some(1.0), "opens unzoomed");

    // The default grid must overflow so the scroll area has positive scroll headroom to re-anchor
    // into (a non-overflowing grid would clamp every re-anchor back to 0 — no observable signal).
    let size = app
        .world()
        .get_resource::<MapEditorSession>()
        .map(MapEditorSession::grid_size);
    assert!(
        size.is_some_and(|s| *s.width() >= 40 && *s.height() >= 40),
        "the default grid must overflow the viewport so the cursor-anchor re-scroll has headroom",
    );

    // Park the cursor OFF the viewport top-left (well into the lower-right of the scroll viewport),
    // so the content point under it is not the scroll origin — re-anchoring it on a zoom-in must
    // push the scroll position positive.
    let Some((vp_size, centre)) = area_geometry(&mut app) else {
        return;
    };
    assert!(
        vp_size.x > 0.0 && vp_size.y > 0.0,
        "the scroll viewport must have a non-zero size to host the off-origin cursor",
    );
    // Three-quarters down/right of the viewport centre — comfortably inside the viewport (gate
    // passes) but far from its top-left corner (so the anchored scroll is clearly positive).
    let cursor = centre + vp_size * 0.25;
    set_cursor(&mut app, cursor);
    assert!(
        cursor_over_canvas(&mut app),
        "the anchor cursor must be over the canvas so the wheel is taken as a zoom",
    );

    let before = canvas_scroll_position(&mut app);
    assert_eq!(
        before,
        Some(Vec2::ZERO),
        "the canvas opens scrolled to the origin (C3 pre-state)",
    );

    // Zoom IN several notches — the cursor-anchored re-layout must re-anchor the scroll about the
    // cursor, producing a positive offset on both axes (content grew, cursor held).
    wheel(&mut app, 3.0);
    assert!(
        current_zoom(&app).is_some_and(|z| *z > 1.0),
        "the wheel must have zoomed in for the anchor to act",
    );
    let after = canvas_scroll_position(&mut app);
    let grew = matches!(
        (before, after),
        (Some(b), Some(a)) if a.x > b.x + f32::EPSILON && a.y > b.y + f32::EPSILON
    );
    assert!(
        grew,
        "cursor-anchored zoom must re-anchor ScrollPosition (it must GROW from 0 with an off-origin \
         cursor) so the content point under the cursor stays put (C3): {before:?} -> {after:?}",
    );
}

/// The single `CanvasRoot`'s `UiTransform` translation, if present.
fn root_translation(app: &mut App) -> Option<(Val, Val)> {
    let world = app.world_mut();
    let mut q = world.query_filtered::<&UiTransform, With<CanvasRoot>>();
    q.iter(world)
        .next()
        .map(|t| (t.translation.x, t.translation.y))
}

/// Drive the width + height size fields to `span × span` and settle.
fn set_grid_span(app: &mut App, span: u8) {
    let (width_field, height_field) = size_fields(app);
    if let (Some(w), Some(h)) = (width_field, height_field) {
        app.world_mut().write_message(NumericFieldCommitted::new(
            w,
            CommittedNumericValue::new(GridSpanInput::new(span)),
        ));
        app.world_mut().write_message(NumericFieldCommitted::new(
            h,
            CommittedNumericValue::new(GridSpanInput::new(span)),
        ));
    }
    settle(app);
}

/// T4a (C2): a SMALL grid (smaller than the viewport) is CENTRED — the `CanvasRoot` `UiTransform`
/// translation is a NON-ZERO `Px` offset on at least one axis.
///
/// Pin-discriminating: reverting the centring leaves the translation at IDENTITY (`0,0`), so the
/// non-zero assert fails. (The exact offset is layout-derived, never pinned.)
#[test]
fn small_grid_is_centred() {
    let mut app = editor_in_editing();
    // Shrink to a tiny grid that is far smaller than the centre column viewport.
    set_grid_span(&mut app, 2);
    settle(&mut app);

    let translation = root_translation(&mut app);
    assert!(
        translation.is_some(),
        "the canvas root must carry a UiTransform (C2)"
    );
    let nonzero = matches!(translation, Some((Val::Px(x), _)) if x > 0.0)
        || matches!(translation, Some((_, Val::Px(y))) if y > 0.0);
    assert!(
        nonzero,
        "a grid smaller than the viewport must be CENTRED via a non-zero UiTransform translation \
         (C2), got {translation:?}",
    );
}

/// T4b (C2): a LARGE grid (overflowing the viewport) is NOT translated on the overflowing axis —
/// the translation is `0` there, so scrolling reaches the whole grid (the GTW-421 trap is avoided).
///
/// Pin-discriminating: a `justify/align: Center` centring (the forbidden approach) would push the
/// overflowing content's start off-screen; the correct translation-based centring leaves `0` on an
/// overflowing axis, which this asserts.
#[test]
fn large_grid_is_not_translated_on_overflow_axis() {
    let mut app = editor_in_editing();
    // The default grid is 60×60 (1440 px) — far larger than the centre column, so it overflows
    // both axes. Re-assert the seeded default before checking.
    let size = app
        .world()
        .get_resource::<MapEditorSession>()
        .map(MapEditorSession::grid_size);
    assert!(
        size.is_some_and(|s| *s.width() >= 40 && *s.height() >= 40),
        "the default grid must overflow the centre column for this test (C2)",
    );
    settle(&mut app);

    // On an overflowing axis the translation must be 0 px (the centring writes `Val::Px(0.0)` there
    // — and `Val::ZERO` is itself `Val::Px(0.0)`), never a positive centring offset.
    let translation = root_translation(&mut app);
    let x_zero = matches!(translation, Some((Val::Px(x), _)) if x.abs() < f32::EPSILON);
    let y_zero = matches!(translation, Some((_, Val::Px(y))) if y.abs() < f32::EPSILON);
    assert!(
        x_zero && y_zero,
        "an overflowing grid must have ZERO centring translation on the overflowing axes so \
         scrolling reaches the whole grid (C2, the GTW-421 trap), got {translation:?}",
    );
}

/// Find the width + height size-field entities by their [`SizeFieldAxis`] markers.
fn size_fields(app: &mut App) -> (Option<Entity>, Option<Entity>) {
    let world = app.world_mut();
    let mut q = world.query::<(Entity, &SizeFieldAxis)>();
    let mut width = None;
    let mut height = None;
    for (entity, axis) in q.iter(world) {
        match axis {
            SizeFieldAxis::Width => width = Some(entity),
            SizeFieldAxis::Height => height = Some(entity),
            SizeFieldAxis::Levels => {}
        }
    }
    (width, height)
}
