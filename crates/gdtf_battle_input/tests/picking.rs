//! GTW-221 (GTW-48 S7): headless integration tests for the cursor->cell picking +
//! the hover-highlight sprite.
//!
//! - AC1 proves `GdtfBattleInputPlugin`'s `build` runs inside the REAL scene stack:
//!   the `GdtfTestAppBuilder` (`MinimalPlugins` + the real `ScenesPlugin` state
//!   machine) descends to `GameState::BattleScape` and the plugin's
//!   `GdtfBattleInputActive` marker is present — the exact `presenter_foundation.rs`
//!   build-ran pattern. It also names the boundary types
//!   (`HoveredCell` / the presenter's `WorldCamera` / `CELL_PX` / `cell_to_world` /
//!   `ActiveLevel` / `gdtf_battle_sim::{Cell, Level, CellLevel}`) so the chain is
//!   compile-proven reachable.
//!
//! - AC2-AC4 drive the picking + highlight systems in a focused headless app: a
//!   `WorldCamera` is SYNTHESIZED at a known transform with a deterministic
//!   orthographic projection (no render pipeline — `Camera.computed` is set in the
//!   test body, mirroring bevy's own `viewport_to_world` unit test), a `Window` +
//!   `PrimaryWindow` is spawned with a known cursor, `ActiveLevel` + the
//!   `BattleInProgress` gate are inserted, then `app.update()` runs the real
//!   systems and the test asserts on `HoveredCell` + the one highlight sprite.
//!
//! Every `app.world_mut()` / cursor / camera mutation is in a TEST BODY — the
//! accepted headless idiom (`bevy-traps.md` #7 carve-out (a)). No function here
//! takes `&mut World`/`&World`.

use bevy::{
    app::App,
    camera::{
        Camera, ComputedCameraValues, OrthographicProjection, Projection, RenderTargetInfo,
        primitives::Frustum, visibility::RenderLayers,
    },
    math::{Vec2, Vec3},
    prelude::*,
    transform::components::GlobalTransform,
    window::{PrimaryWindow, Window, WindowResolution},
};
use gdtf_app::test_support::{AppState, GameState, RunningState};
use gdtf_battle_input::{
    GdtfBattleInputActive, GdtfBattleInputPlugin, HoverHighlight, HoveredCell, world_to_cell,
};
use gdtf_battle_presenter::{ActiveLevel, CELL_PX, WORLD_RENDER_LAYER, WorldCamera, cell_to_world};
use gdtf_battle_sim::{BattleInProgress, Cell, CellLevel, Level, tuning::CombatTuning};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::default_theme;

/// A budget large enough to drive the deep walk into the battlescape, bounded so a
/// machine that never reaches the predicate fails instead of hanging.
const BUDGET: u32 = 96;

/// The synthetic window/camera render-target size (physical px), large enough that a
/// cursor near its centre unprojects to an in-grid cell.
const TARGET_SIZE: Vec2 = Vec2::new(1280.0, 720.0);

// ---------------------------------------------------------------------------------
// AC1 — the plugin build ran inside the real scene stack.
// ---------------------------------------------------------------------------------

/// Reads the current [`GameState`] if it is active.
fn game_state(app: &App) -> Option<GameState> {
    app.world()
        .get_resource::<State<GameState>>()
        .map(|state| *state.get())
}

/// Reads the current [`RunningState`] if it is active.
fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

/// Builds the headless app at [`AppState::Running`], seeding the persistent `Load`
/// resources the machine needs to traverse `Load` under `MinimalPlugins` (the
/// `presenter_foundation` / `battle_bootstrap` precedent): `default_theme()` +
/// `CombatTuning`. No `LoadedSituation` is seeded — Generation falls back to the
/// default (empty) situation, sufficient to reach `BattleScape`.
fn scene_stack_app() -> App {
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Running)
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app
}

/// Drives the app from [`AppState::Running`] down to the first update on which
/// [`GameState::BattleScape`] is active. Returns whether it was reached.
fn drive_to_battlescape(app: &mut App) -> bool {
    let reached_menu = advance_until(
        app,
        |app| running_state(app) == Some(RunningState::Menu),
        BUDGET,
    );
    if !reached_menu {
        return false;
    }
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Options);
    advance_until(
        app,
        |app| game_state(app) == Some(GameState::BattleScape),
        BUDGET,
    )
}

/// AC1 — `GameBattleScapeScenePlugin` adds `GdtfBattleInputPlugin`, and its `build`
/// actually runs inside the real scene stack: descending to `GameState::BattleScape`
/// leaves the `GdtfBattleInputActive` marker (and the `HoveredCell` resource the
/// plugin initialises) present in the world.
#[test]
fn battlescape_scene_runs_the_input_plugin_build() {
    let mut app = scene_stack_app();
    assert!(
        drive_to_battlescape(&mut app),
        "the walk should descend to GameState::BattleScape within {BUDGET} updates; last observed \
         GameState was {:?}",
        game_state(&app),
    );
    assert_eq!(
        game_state(&app),
        Some(GameState::BattleScape),
        "the walk must rest inside GameState::BattleScape",
    );
    assert!(
        app.world()
            .get_resource::<GdtfBattleInputActive>()
            .is_some(),
        "GdtfBattleInputPlugin::build must have run inside the real scene stack — the \
         GdtfBattleInputActive marker is present once BattleScape is active",
    );
    assert!(
        app.world().get_resource::<HoveredCell>().is_some(),
        "GdtfBattleInputPlugin must initialise the HoveredCell resource on build",
    );
}

// ---------------------------------------------------------------------------------
// AC2-AC4 — focused picking/highlight harness with a synthesized camera + cursor.
// ---------------------------------------------------------------------------------

/// Builds a deterministic [`Camera`] whose `viewport_to_world_2d` succeeds WITHOUT a
/// render pipeline, mirroring bevy's own `viewport_to_world` unit test: set the
/// render-target info + viewport, run the projection's `update`, and store its
/// clip-from-view matrix in `computed`.
fn synthetic_camera() -> Camera {
    // Compute the clip-from-view matrix for a render area the size of the target,
    // then assemble the whole `computed` block in one initializer.
    let mut projection = Projection::Orthographic(OrthographicProjection::default_2d());
    projection.update(TARGET_SIZE.x, TARGET_SIZE.y);
    Camera {
        computed: ComputedCameraValues {
            target_info: Some(RenderTargetInfo {
                physical_size: TARGET_SIZE.as_uvec2(),
                scale_factor:  1.0,
            }),
            clip_from_view: projection.get_clip_from_view(),
            ..ComputedCameraValues::default()
        },
        ..Camera::default()
    }
}

/// Builds a focused headless picking app: `MinimalPlugins` + the
/// `GdtfBattleInputPlugin`, the presenter-owned `ActiveLevel`, the `BattleInProgress`
/// gate, and a synthesized `WorldCamera` + `Window`/`PrimaryWindow`. The cursor is
/// left unset (off-window) until a test sets it.
fn picking_app(active_level: Level) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(GdtfBattleInputPlugin);
    // The presenter normally owns ActiveLevel (init in TopDownRendererPlugin); the
    // focused harness inserts it directly so picking has a level to band on.
    app.world_mut().insert_resource(ActiveLevel(active_level));
    app.world_mut().insert_resource(BattleInProgress);

    // The synthetic world camera: a deterministic projection at the identity
    // transform. The `Projection`/`Frustum` are required-component siblings of a real
    // Camera2d; spawning them keeps the camera entity well-formed.
    app.world_mut().spawn((
        Camera2d,
        WorldCamera,
        synthetic_camera(),
        GlobalTransform::IDENTITY,
        Projection::Orthographic(OrthographicProjection::default_2d()),
        Frustum::default(),
    ));

    // The single primary window with a known resolution (so a cursor inside it is
    // reported by `cursor_position()`).
    app.world_mut().spawn((
        Window {
            resolution: WindowResolution::new(TARGET_SIZE.x as u32, TARGET_SIZE.y as u32),
            ..default()
        },
        PrimaryWindow,
    ));
    app
}

/// Sets the primary window's cursor position in logical px (or clears it).
fn set_cursor(app: &mut App, position: Option<Vec2>) {
    let mut windows = app.world_mut().query::<&mut Window>();
    for mut window in windows.iter_mut(app.world_mut()) {
        window.set_cursor_position(position);
    }
}

/// Reads the current `HoveredCell` value.
fn hovered(app: &App) -> Option<CellLevel> {
    app.world().get_resource::<HoveredCell>().and_then(|h| **h)
}

/// The single hover-highlight sprite's translation + visibility, if it exists.
fn highlight_state(app: &mut App) -> Option<(Vec3, Visibility)> {
    let mut q = app
        .world_mut()
        .query_filtered::<(&Transform, &Visibility), With<HoverHighlight>>();
    let mut iter = q.iter(app.world());
    let first = iter.next().map(|(t, v)| (t.translation, *v));
    // Exactly one (or zero) highlight entity is the invariant; the caller asserts the
    // count separately, but a second match here would be a bug, so collapse to first.
    first
}

/// Counts the hover-highlight sprites in the world.
fn highlight_count(app: &mut App) -> usize {
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, With<HoverHighlight>>();
    q.iter(app.world()).count()
}

/// The camera's world unprojection of `cursor` — what the picking system computes
/// internally. The test re-derives the expected cell from this with the documented
/// inverse, so it never hardcodes the world math.
fn unproject(app: &mut App, cursor: Vec2) -> Option<Vec2> {
    let mut q = app
        .world_mut()
        .query_filtered::<(&Camera, &GlobalTransform), With<WorldCamera>>();
    let (camera, transform) = q.iter(app.world()).next()?;
    camera.viewport_to_world_2d(transform, cursor).ok()
}

/// AC2 — given a synthesized camera + cursor, the picking maps the cursor to the
/// `CellLevel` the test computes INDEPENDENTLY from `viewport_to_world_2d` + the
/// documented inverse, and writes `HoveredCell(Some(..))`.
#[test]
fn picking_resolves_the_cursor_to_the_documented_cell() {
    let level = Level::new(0);
    let mut app = picking_app(level);

    // A cursor offset from the window centre so it lands on a non-origin in-grid cell.
    // Centre maps to world (0,0) = cell (0,0). An in-grid cell needs world.x >= 0 and
    // world.y <= 0; screen-y grows DOWNWARD while world-y grows UPWARD, so the cursor
    // shifts RIGHT (+screen x) and DOWN (+screen y) to reach the positive-x / positive-
    // row region.
    let cursor = TARGET_SIZE * 0.5 + Vec2::new(40.0, 32.0);
    set_cursor(&mut app, Some(cursor));
    app.update();

    // Compute the expected cell INDEPENDENTLY from the camera unprojection + the
    // documented inverse (cx = floor(world.x/CELL_PX), cy = floor(-world.y/CELL_PX)).
    let world = unproject(&mut app, cursor);
    assert!(
        world.is_some(),
        "the synthetic camera must unproject the cursor"
    );
    let Some(world) = world else { return };
    let expected = world_to_cell(world, level);
    assert!(
        expected.is_some(),
        "the chosen cursor must land inside the grid (world {world:?})",
    );

    assert_eq!(
        hovered(&app),
        expected,
        "HoveredCell must equal the documented inverse of the camera unprojection",
    );
}

/// AC3 — `HoveredCell` resolves fail-closed to `None` (no panic) when: the cursor is
/// off-window; the computed cell is outside the 60×60 grid; and there is no world
/// camera. Each case drives `app.update()` and asserts `None`.
#[test]
fn picking_fails_closed_to_none() {
    let level = Level::new(0);

    // (a) No cursor — `cursor_position()` is None.
    {
        let mut app = picking_app(level);
        set_cursor(&mut app, None);
        app.update();
        assert_eq!(
            hovered(&app),
            None,
            "an off-window cursor (no cursor_position) must resolve HoveredCell to None",
        );
    }

    // (b) A cursor whose world->cell lands OUTSIDE the grid. The window centre maps to
    // world (0,0) = cell (0,0); shifting LEFT of centre pushes world.x negative, so
    // cell.x floors below 0 — out of grid.
    {
        let mut app = picking_app(level);
        let cursor = TARGET_SIZE * 0.5 - Vec2::new(64.0, 0.0);
        set_cursor(&mut app, Some(cursor));
        app.update();
        // Confirm the chosen cursor really is off-grid via the documented inverse.
        let world = unproject(&mut app, cursor);
        let Some(world) = world else {
            return;
        };
        assert_eq!(
            world_to_cell(world, level),
            None,
            "the chosen left-of-origin cursor must be off-grid (world {world:?})",
        );
        assert_eq!(
            hovered(&app),
            None,
            "an off-grid cursor must resolve HoveredCell to None",
        );
    }

    // (c) No world camera — despawn it, set an in-window cursor, update.
    {
        let mut app = picking_app(level);
        let cameras: Vec<Entity> = {
            let mut q = app
                .world_mut()
                .query_filtered::<Entity, With<WorldCamera>>();
            q.iter(app.world()).collect()
        };
        for camera in cameras {
            app.world_mut().entity_mut(camera).despawn();
        }
        set_cursor(&mut app, Some(TARGET_SIZE * 0.5));
        app.update();
        assert_eq!(
            hovered(&app),
            None,
            "with no WorldCamera the picking must resolve HoveredCell to None",
        );
    }
}

/// AC4 — exactly ONE hover-highlight sprite is drawn at `cell_to_world(hovered)`,
/// FOLLOWS `HoveredCell` across two distinct in-grid cursor positions with no
/// duplicate accumulation, and is hidden when `HoveredCell` becomes `None`. The
/// highlight is sized to one cell and on the world render layer.
#[test]
fn highlight_follows_the_hovered_cell_and_hides_on_none() {
    let level = Level::new(0);
    let mut app = picking_app(level);

    // First in-grid cursor (shifted right + down — see the AC2 test for the
    // screen->world sign reasoning).
    let cursor_a = TARGET_SIZE * 0.5 + Vec2::new(40.0, 32.0);
    set_cursor(&mut app, Some(cursor_a));
    app.update();
    let cell_a = hovered(&app);
    assert!(cell_a.is_some(), "cursor A must resolve to an in-grid cell");
    let Some(cell_a) = cell_a else { return };
    assert_eq!(
        highlight_count(&mut app),
        1,
        "exactly one highlight sprite exists after the first hover",
    );
    let state_a = highlight_state(&mut app);
    assert_eq!(
        state_a,
        Some((
            cell_to_world(Cell::new(cell_a.x, cell_a.y), level),
            Visibility::Visible,
        )),
        "the highlight must be visible at cell_to_world(hovered A)",
    );

    // A DIFFERENT in-grid cursor — the highlight must move, not duplicate.
    let cursor_b = TARGET_SIZE * 0.5 + Vec2::new(200.0, 160.0);
    set_cursor(&mut app, Some(cursor_b));
    app.update();
    let cell_b = hovered(&app);
    assert!(cell_b.is_some(), "cursor B must resolve to an in-grid cell");
    let Some(cell_b) = cell_b else { return };
    assert_ne!(
        cell_b, cell_a,
        "cursor B must land on a different cell than A"
    );
    assert_eq!(
        highlight_count(&mut app),
        1,
        "still exactly one highlight sprite after the second hover (no duplicate)",
    );
    let state_b = highlight_state(&mut app);
    assert_eq!(
        state_b,
        Some((
            cell_to_world(Cell::new(cell_b.x, cell_b.y), level),
            Visibility::Visible,
        )),
        "the highlight must have MOVED to cell_to_world(hovered B)",
    );

    // The render-layer + sizing recipe: the one highlight draws on the world layer at
    // one-cell size.
    assert!(
        highlight_on_world_layer_at_cell_size(&mut app),
        "the highlight must be CELL_PX-sized on the WORLD_RENDER_LAYER",
    );

    // Off-grid cursor => HoveredCell None => the highlight hides (still one entity).
    let cursor_off = TARGET_SIZE * 0.5 - Vec2::new(64.0, 0.0);
    set_cursor(&mut app, Some(cursor_off));
    app.update();
    assert_eq!(
        hovered(&app),
        None,
        "the off-grid cursor must clear HoveredCell to None",
    );
    assert_eq!(
        highlight_count(&mut app),
        1,
        "the highlight entity persists (hidden, not duplicated) when nothing is hovered",
    );
    assert_eq!(
        highlight_state(&mut app).map(|(_, v)| v),
        Some(Visibility::Hidden),
        "the highlight must be hidden when HoveredCell is None",
    );
}

/// Whether the one highlight sprite is `CELL_PX`-sized and on the world render layer.
fn highlight_on_world_layer_at_cell_size(app: &mut App) -> bool {
    let mut q = app
        .world_mut()
        .query_filtered::<(&Sprite, &RenderLayers), With<HoverHighlight>>();
    let world_layer = RenderLayers::layer(WORLD_RENDER_LAYER);
    q.iter(app.world()).all(|(sprite, layers)| {
        sprite.custom_size == Some(Vec2::splat(CELL_PX)) && layers.intersects(&world_layer)
    })
}
