//! GTW-221 (GTW-48 S7) + GTW-251: headless integration tests for the cursor->cell
//! picking and the message-driven hover-highlight EMIT.
//!
//! - AC1 (build-ran) proves `GdtfBattleInputPlugin`'s `build` runs inside the REAL
//!   scene stack: the `GdtfTestAppBuilder` (`MinimalPlugins` + the real `ScenesPlugin`
//!   state machine) descends to `GameState::BattleScape` and the plugin's
//!   `GdtfBattleInputActive` marker is present — the exact `presenter_foundation.rs`
//!   build-ran pattern. It also names the boundary types
//!   (`InspectTarget` / the presenter's `WorldCamera` / `CELL_PX` / `cell_to_world` /
//!   `ActiveLevel` / `gdtf_battle_sim::{Cell, Level, CellLevel}`) so the chain is
//!   compile-proven reachable.
//!
//! - The picking tests drive `pick_hovered_cell` in a focused headless app: a
//!   `WorldCamera` is SYNTHESIZED at a known transform with a deterministic
//!   orthographic projection (no render pipeline — `Camera.computed` is set in the
//!   test body, mirroring bevy's own `viewport_to_world` unit test), a `Window` +
//!   `PrimaryWindow` is spawned with a known cursor, `ActiveLevel` + the
//!   `BattleInProgress` gate are inserted, then `app.update()` runs the real
//!   systems and the test asserts on `InspectTarget`.
//!
//! - GTW-251 AC1 (picker emits a request matching `InspectTarget`): a probe drains the
//!   presenter-owned `HighlightRequest` buffer AFTER `emit_highlight_request` and
//!   asserts the emitted request equals the resolved `InspectTarget` (`Some(cell)` when
//!   hovered, `None` when off-grid). The DRAWING moved to the presenter (GTW-251), so
//!   the old input-side draw test migrated there (`tests/highlight_draw.rs`).
//!
//! Every `app.world_mut()` / cursor / camera mutation is in a TEST BODY — the
//! accepted headless idiom (`bevy-traps.md` #7 carve-out (a)). No function here
//! takes `&mut World`/`&World`.

use bevy::{
    app::App,
    camera::{
        Camera, ComputedCameraValues, OrthographicProjection, Projection, RenderTargetInfo,
        Viewport, primitives::Frustum,
    },
    input::ButtonInput,
    math::{URect, UVec2, Vec2},
    prelude::*,
    transform::components::GlobalTransform,
    window::{PrimaryWindow, Window, WindowResolution},
};
use gdtf_app::test_support::{AppState, GameState, RunningState};
use gdtf_battle_input::{
    GdtfBattleInputActive, GdtfBattleInputPlugin, InspectTarget, emit_highlight_request,
    world_to_cell,
};
use gdtf_battle_presenter::{ActiveLevel, HighlightRequest, WorldCamera};
use gdtf_battle_sim::{
    BattleInProgress, CellLevel, Faction, Level, OccupancyGrid, PlayerFaction, TerrainKind,
    VerticalLinkGraph, acts::MoveRequested, tuning::CombatTuning,
};
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
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
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
/// leaves the `GdtfBattleInputActive` marker (and the `InspectTarget` resource the
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
        app.world().get_resource::<InspectTarget>().is_some(),
        "GdtfBattleInputPlugin must initialise the InspectTarget resource on build",
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
    app.world_mut()
        .insert_resource(ActiveLevel::new(active_level));
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

/// Reads the current `InspectTarget` live hovered cell.
fn hovered(app: &App) -> Option<CellLevel> {
    app.world()
        .get_resource::<InspectTarget>()
        .and_then(InspectTarget::hovered)
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
/// documented inverse, and writes `InspectTarget(Some(..))`.
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
        "InspectTarget must equal the documented inverse of the camera unprojection",
    );
}

/// AC3 — `InspectTarget` resolves fail-closed to `None` (no panic) when: the cursor is
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
            "an off-window cursor (no cursor_position) must resolve InspectTarget to None",
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
            "an off-grid cursor must resolve InspectTarget to None",
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
            "with no WorldCamera the picking must resolve InspectTarget to None",
        );
    }
}

// ---------------------------------------------------------------------------------
// GTW-251 AC1 — the picker EMITS a `HighlightRequest` matching `InspectTarget`.
// ---------------------------------------------------------------------------------

/// The `HighlightRequest`s the probe drained this run (test-only framework plumbing —
/// a `Vec` collector so the assert reads exactly what `emit_highlight_request` wrote).
#[derive(Resource, Default)]
struct HighlightProbe(Vec<HighlightRequest>);

/// Adds a probe that drains `Messages<HighlightRequest>` AFTER `emit_highlight_request`
/// so it collects every request the emitter wrote this update (its own `MessageReader`
/// cursor, independent of the presenter's `draw_highlight_on_request` reader).
fn add_highlight_probe(app: &mut App) {
    app.insert_resource(HighlightProbe::default());
    app.add_systems(
        Update,
        (|mut r: MessageReader<HighlightRequest>, mut p: ResMut<HighlightProbe>| {
            p.0.extend(r.read().copied());
        })
        .after(emit_highlight_request),
    );
}

/// The requests the probe collected on the latest update (cleared each update because
/// the probe `extend`s — the test reads them right after the relevant `update()`).
fn requests(app: &App) -> Vec<HighlightRequest> {
    app.world()
        .get_resource::<HighlightProbe>()
        .map(|p| p.0.clone())
        .unwrap_or_default()
}

/// GTW-251 AC1 + GTW-268 — the picker emits a `HighlightRequest` matching `InspectTarget`,
/// GATED on occupancy: an in-grid cursor over a BLOCKING (or occupied) cell emits
/// `Some(that cell)`; over BARE FLOOR it emits `None` (GTW-268: gangers + objects only);
/// off-grid it emits `None`. The DRAWING is the presenter's (`tests/highlight_draw.rs`);
/// here we pin the input EMIT.
#[test]
fn picker_emits_highlight_request_matching_hovered_cell() {
    let level = Level::new(0);
    let mut app = picking_app(level);
    // GTW-268 — the emit now gates on the `OccupancyGrid`; seed one (empty) for this harness.
    app.world_mut().insert_resource(OccupancyGrid::default());
    add_highlight_probe(&mut app);

    // An in-grid cursor (shifted right + down — see the AC2 picking test for the
    // screen->world sign reasoning).
    let cursor = TARGET_SIZE * 0.5 + Vec2::new(40.0, 32.0);
    set_cursor(&mut app, Some(cursor));
    app.update();

    let cell = hovered(&app);
    assert!(cell.is_some(), "the in-grid cursor must resolve a cell");
    let Some(resolved) = cell else { return };

    // GTW-268 — over BARE FLOOR the emit is gated to None even though a cell is hovered.
    assert_eq!(
        requests(&app),
        vec![HighlightRequest::new(None)],
        "a bare-floor in-grid cell must emit HighlightRequest(None) (GTW-268)",
    );

    // Mark the hovered cell BLOCKING (an object): the emit now carries Some(that cell).
    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.set_terrain(resolved, TerrainKind::Cover);
    }
    app.world_mut().resource_mut::<HighlightProbe>().0.clear();
    app.update();
    assert_eq!(
        requests(&app),
        vec![HighlightRequest::new(cell)],
        "over a blocking cell the picker must emit exactly one HighlightRequest = Some(cell)",
    );

    // Now move the cursor off-grid: InspectTarget becomes None and the emitted request
    // must follow it to None.
    let cursor_off = TARGET_SIZE * 0.5 - Vec2::new(64.0, 0.0);
    set_cursor(&mut app, Some(cursor_off));
    // Reset the probe so we read only THIS update's emit.
    app.world_mut().resource_mut::<HighlightProbe>().0.clear();
    app.update();

    assert_eq!(
        hovered(&app),
        None,
        "the off-grid cursor clears InspectTarget"
    );
    assert_eq!(
        requests(&app),
        vec![HighlightRequest::new(None)],
        "the picker must emit HighlightRequest(None) when nothing is hovered",
    );
}

// ---------------------------------------------------------------------------------
// GTW-286 (Bug D) — the world-click/pick path is GATED to the map viewport rect: a
// cursor over a margin / UI panel resolves `InspectTarget` to None, so no move/select/
// fire/reticle reaches through the UI even though `viewport_to_world_2d` would happily
// EXTRAPOLATE it into a valid in-grid cell.
// ---------------------------------------------------------------------------------

/// The faction the player controls in these tests (matches the seeded `PlayerFaction`).
const PLAYER_FACTION: Faction = Faction::new(0);

/// A central viewport SUB-RECT of the synthetic target: inset 320px left/right and
/// 180px top/bottom, so a cursor in the BOTTOM margin (below `max.y`) lies OUTSIDE it.
/// Physical px == logical px here (the synthetic camera's `scale_factor` is `1.0`).
const VIEWPORT_RECT: URect = URect {
    min: UVec2::new(320, 180),
    max: UVec2::new(960, 540),
};

/// Sets the world camera's `viewport` to a sub-rect, so `logical_viewport_rect()` reports
/// that rect (not the full target) — the map sub-rect the GTW-286 gate confines clicks to.
fn set_world_viewport(app: &mut App, rect: URect) {
    let mut cameras = app
        .world_mut()
        .query_filtered::<&mut Camera, With<WorldCamera>>();
    for mut camera in cameras.iter_mut(app.world_mut()) {
        camera.viewport = Some(Viewport {
            physical_position: rect.min,
            physical_size: rect.size(),
            ..Viewport::default()
        });
    }
}

/// Builds a picking app wired for the full MOVE path: a viewport sub-rect, the
/// `OccupancyGrid` / `CombatTuning` / `PlayerFaction` / `ButtonInput<MouseButton>` the
/// `left_click_act` run condition needs, plus a pre-selected PLAYER-faction ganger (with
/// NO firing components, so FIRE fails closed and a click on an empty cell is a MOVE) and a
/// `MoveProbe` draining `Messages<MoveRequested>` after `left_click_act`.
fn move_path_app(active_level: Level) -> App {
    let mut app = picking_app(active_level);
    set_world_viewport(&mut app, VIEWPORT_RECT);
    app.world_mut().insert_resource(OccupancyGrid::default());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut()
        .insert_resource(PlayerFaction::new(PLAYER_FACTION));
    // GTW-356: the shared left-click decision reads `Res<VerticalLinkGraph>` (the OQ-4
    // link-tile gate) via `LeftClickReads`, and `battle_act_gate()` now gates the click systems
    // on it — seed an empty graph (no links, so every move target is a non-link tile).
    app.world_mut()
        .insert_resource(VerticalLinkGraph::default());
    app.world_mut()
        .insert_resource(ButtonInput::<MouseButton>::default());

    // A player-faction ganger with no firing components: FIRE fails closed (no
    // `ShooterFireData`), so a click on an empty in-bounds cell is a MOVE. Pre-select it.
    let ganger = app.world_mut().spawn(PLAYER_FACTION).id();
    app.world_mut()
        .insert_resource(gdtf_battle_input::SelectedShooter::new(ganger));

    app.insert_resource(MoveProbe::default());
    app.add_systems(
        Update,
        (|mut r: MessageReader<MoveRequested>, mut p: ResMut<MoveProbe>| {
            p.0.extend(r.read().copied());
        })
        // After the drain so the probe sees the SAME update's emitted `MoveRequested`.
        .after(gdtf_battle_input::dispatch_act_intents),
    );
    app
}

/// The `MoveRequested` messages the probe drained (test-only framework plumbing).
#[derive(Resource, Default)]
struct MoveProbe(Vec<MoveRequested>);

/// The move requests the probe collected so far.
fn move_requests(app: &App) -> Vec<MoveRequested> {
    app.world()
        .get_resource::<MoveProbe>()
        .map(|p| p.0.clone())
        .unwrap_or_default()
}

/// Presses (just-pressed edge) the left mouse button.
fn press_left(app: &mut App) {
    if let Some(mut mouse) = app
        .world_mut()
        .get_resource_mut::<ButtonInput<MouseButton>>()
    {
        mouse.press(MouseButton::Left);
    }
}

/// Releases + clears the mouse edges so the NEXT `press_left` is a fresh just-pressed (under
/// the headless harness no `InputPlugin` clears the edges per frame, so a two-click sequence —
/// the GTW-356 target-then-commit flow — must clear between presses).
fn clear_mouse(app: &mut App) {
    if let Some(mut mouse) = app
        .world_mut()
        .get_resource_mut::<ButtonInput<MouseButton>>()
    {
        mouse.release(MouseButton::Left);
        mouse.clear();
    }
}

/// GTW-286 INSIDE — a cursor INSIDE the viewport sub-rect over a valid in-grid cell
/// resolves `InspectTarget` to that cell AND a TWO-CLICK there (GTW-356: click-1 targets,
/// click-2 commits) emits a `MoveRequested`. (Guards against over-suppression: the gate must
/// NOT reject an in-viewport cursor.)
#[test]
fn two_click_inside_the_viewport_resolves_a_cell_and_moves() {
    let level = Level::new(0);
    let mut app = move_path_app(level);

    // A cursor INSIDE the viewport sub-rect, offset down+right of its centre so it lands
    // on a non-origin in-grid cell (screen-y down -> world-y down -> positive cell row;
    // screen-x right -> positive cell column).
    let viewport_centre = VIEWPORT_RECT.center().as_vec2();
    let cursor = viewport_centre + Vec2::new(20.0, 16.0);

    // Update 1: the picker resolves `InspectTarget` from the in-viewport cursor.
    set_cursor(&mut app, Some(cursor));
    app.update();

    // The chosen cursor lands on an in-grid cell (independent of the gate's outcome).
    let world = unproject(&mut app, cursor);
    let Some(world) = world else {
        unreachable!("the synthetic camera must unproject the in-viewport cursor");
    };
    let expected = world_to_cell(world, level);
    assert!(
        expected.is_some(),
        "the chosen in-viewport cursor must land inside the grid (world {world:?})",
    );
    assert_eq!(
        hovered(&app),
        expected,
        "an INSIDE-viewport cursor must resolve InspectTarget to its cell (gate must not over-suppress)",
    );

    // Update 2: click-1 -> `left_click_act` (which reads last update's InspectTarget) SETS
    // the move target (GTW-356); NO MoveRequested yet.
    press_left(&mut app);
    app.update();
    assert!(
        move_requests(&app).is_empty(),
        "click-1 on an in-viewport empty cell SETS the target — no MoveRequested yet (GTW-356)",
    );

    // Update 3: click-2 on the SAME cell (cursor unmoved) COMMITS — exactly one MoveRequested.
    clear_mouse(&mut app);
    press_left(&mut app);
    app.update();
    assert_eq!(
        move_requests(&app).len(),
        1,
        "click-2 on the same in-viewport cell must commit exactly one MoveRequested (GTW-356)",
    );
}

/// GTW-286 MARGIN — a cursor in the BOTTOM margin (below the viewport's `max.y`) at a
/// screen position whose EXTRAPOLATED world point still floors to an in-grid 0..60 cell
/// (proving the OLD ungated code would have moved) resolves `InspectTarget` to None AND a
/// left-click there emits NO `MoveRequested`. Pin-discriminating: RED before the gate
/// (the extrapolated cell is in-grid -> a MOVE), GREEN after.
#[test]
fn click_in_the_bottom_margin_resolves_none_and_does_not_move() {
    let level = Level::new(0);
    let mut app = move_path_app(level);

    // A cursor in the BOTTOM margin: same column as the viewport centre, but BELOW
    // `max.y` (in the action-bar margin). It is just past the bottom edge, so
    // `viewport_to_world_2d` extrapolates only slightly past the bottom row -> still an
    // in-grid cell (what the OLD code would have moved on).
    let centre_x = VIEWPORT_RECT.center().as_vec2().x;
    let margin_y = VIEWPORT_RECT.max.y as f32 + 4.0;
    let cursor = Vec2::new(centre_x, margin_y);

    // Prove the OLD code's premise: the EXTRAPOLATED world point of this margin cursor
    // still floors to an in-grid cell (so the gate, not off-grid, is what suppresses it).
    let world = unproject(&mut app, cursor);
    let Some(world) = world else {
        unreachable!("the synthetic camera must unproject the margin cursor");
    };
    assert!(
        world_to_cell(world, level).is_some(),
        "the margin cursor's extrapolated world point must floor to an IN-GRID cell \
         (world {world:?}) — otherwise the test would pass for the wrong reason",
    );

    // Update 1: the picker must GATE the margin cursor (outside the viewport rect) to None.
    set_cursor(&mut app, Some(cursor));
    app.update();
    assert_eq!(
        hovered(&app),
        None,
        "a cursor in the bottom margin (outside the viewport rect) must resolve InspectTarget to \
         None — even though its extrapolated cell is in-grid (GTW-286 gate)",
    );

    // Update 2 + 3: TWO Left presses now find nothing hovered -> no MOVE through the UI margin
    // (GTW-356: even the two-click target-then-commit never fires, because the gate resolves
    // the margin cursor to None so click-1 can never set a target). If the gate were removed,
    // the extrapolated in-grid cell would be targeted then committed — so two clicks make this
    // pin-discriminating against the two-click flow, not just the old single-click move.
    press_left(&mut app);
    app.update();
    clear_mouse(&mut app);
    press_left(&mut app);
    app.update();
    assert!(
        move_requests(&app).is_empty(),
        "a left-click in the bottom margin must emit NO MoveRequested (no move through the UI)",
    );
}
