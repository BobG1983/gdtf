//! World-viewport-scoped picking: margin clicks resolve `None`, UI panels
//! absorb clicks (GTW-286 / GTW-380).

use bevy::{
    app::App,
    camera::{Camera, Viewport, visibility::InheritedVisibility},
    input::ButtonInput,
    math::{URect, UVec2, Vec2},
    prelude::*,
    ui::{ComputedNode, UiGlobalTransform},
};
use gdtf_battle_input::world_to_cell;
use gdtf_battle_presenter::WorldCamera;
use gdtf_battle_sim::{
    Faction, Level, OccupancyGrid, PlayerFaction, VerticalLinkGraph, acts::MoveRequested,
    tuning::CombatTuning,
};
use gdtf_test_utils::{MessageProbePlugin, clear_mouse, press_left, probed};

use super::harness::*;

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

    // The generic GTW-576 message probe — its `Last`-schedule drain observes the same
    // update's emitted `MoveRequested` with its own reader cursor.
    app.add_plugins(MessageProbePlugin::<MoveRequested>::default());
    app
}

/// The move requests the probe collected so far.
fn move_requests(app: &App) -> Vec<MoveRequested> {
    probed::<MoveRequested>(app)
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

// ---------------------------------------------------------------------------------
// GTW-380 (panel click-through) — the UI ABSORBS clicks over its own nodes: a cursor
// over a HUD panel (an absolute OVERLAY drawn INSIDE the world viewport, so the GTW-286
// viewport gate does NOT exclude it) resolves `InspectTarget` to None, so the board
// cell-picker / move never sees the click. Pin-discriminating: the SAME cursor WITHOUT a
// panel under it still moves (proving the gate suppresses only the over-UI case, not the
// in-viewport board click).
// ---------------------------------------------------------------------------------

/// Spawns a synthetic VISIBLE UI panel covering the screen rect `center ± size/2`
/// (PHYSICAL px; `scale_factor` is `1.0` here so logical == physical). Under
/// `MinimalPlugins` there is NO `ui_layout_system` to compute these, so the test body
/// synthesizes the EXACT three components the over-UI gate hit-tests — `ComputedNode`
/// (its `size`), `UiGlobalTransform` (its screen-space center), and `InheritedVisibility`
/// — mirroring how `synthetic_camera` hand-builds a camera with no render pipeline (the
/// accepted headless idiom, `bevy-traps.md` #7 carve-out (a)). It deliberately carries NO
/// `Interaction`, exactly like the real bare-`Node` status / inspect panels, so the test
/// proves the gate catches panels an `Interaction`-only check would MISS.
fn spawn_ui_panel(app: &mut App, center: Vec2, size: Vec2) {
    app.world_mut().spawn((
        ComputedNode {
            size,
            ..ComputedNode::default()
        },
        UiGlobalTransform::from_translation(center),
        InheritedVisibility::VISIBLE,
    ));
}

/// GTW-380 — a TWO-CLICK over a HUD panel (an overlay INSIDE the viewport) is ABSORBED:
/// `InspectTarget` resolves to None and NO `MoveRequested` is emitted, even though the
/// cursor is over a valid in-grid, in-viewport cell that would otherwise MOVE. The SAME
/// cursor with NO panel under it (the `two_click_inside_the_viewport_resolves_a_cell_and_moves`
/// sibling) DOES move — so this is pin-discriminating: it fails (the click falls through to a
/// MOVE) without the over-UI gate, and passes with it.
#[test]
fn two_click_over_a_panel_is_absorbed_and_does_not_move() {
    let level = Level::new(0);
    let mut app = move_path_app(level);

    // The SAME in-viewport, in-grid cursor the moving sibling test uses (offset down+right
    // of the viewport centre so it lands on a non-origin in-grid cell).
    let viewport_centre = VIEWPORT_RECT.center().as_vec2();
    let cursor = viewport_centre + Vec2::new(20.0, 16.0);

    // Confirm the premise: this cursor IS inside the viewport rect AND its world point floors
    // to an in-grid cell (so the GTW-286 viewport gate does NOT suppress it — only the GTW-380
    // panel gate can). Otherwise the test would pass for the wrong reason.
    assert!(
        VIEWPORT_RECT.as_rect().contains(cursor),
        "the chosen cursor must lie INSIDE the viewport rect (so only the panel gate suppresses it)",
    );
    let world = unproject(&mut app, cursor);
    let Some(world) = world else {
        unreachable!("the synthetic camera must unproject the in-viewport cursor");
    };
    assert!(
        world_to_cell(world, level).is_some(),
        "the chosen cursor's world point must floor to an IN-GRID cell (world {world:?}) — \
         so without the panel it WOULD move",
    );

    // A VISIBLE UI panel centered on the cursor (200x120 px), covering it — the HUD overlay.
    spawn_ui_panel(&mut app, cursor, Vec2::new(200.0, 120.0));

    // Update 1: the picker must resolve the over-panel cursor to None (UI absorbs it).
    set_cursor(&mut app, Some(cursor));
    app.update();
    assert_eq!(
        hovered(&app),
        None,
        "a cursor over a HUD panel must resolve InspectTarget to None — the UI absorbs the \
         click (GTW-380), even though the cell underneath is in-grid + in-viewport",
    );

    // Update 2 + 3: a two-click over the panel finds nothing hovered -> no MOVE through the UI.
    // (GTW-356: even the two-click target-then-commit never fires, because the gate resolves the
    // over-panel cursor to None so click-1 can never set a target.)
    press_left(&mut app);
    app.update();
    clear_mouse(&mut app);
    press_left(&mut app);
    app.update();
    assert!(
        move_requests(&app).is_empty(),
        "a left-click over a HUD panel must emit NO MoveRequested — the click must not fall \
         through to the board (GTW-380)",
    );
}
