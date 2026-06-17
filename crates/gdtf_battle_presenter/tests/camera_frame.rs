//! GTW-249 / GTW-271: headless camera-LOGIC tests for the battle-start frame-on-units +
//! the bounds clamp (incl. the GTW-271 viewport-aware fallback half-extent).
//!
//! - AC3 (`frame_camera_on_units`): with a `WorldCamera` + >=2 player-faction gangers at
//!   known cells (+ `PlayerFaction` + `BattleInProgress`), one update centres the camera
//!   on the player gangers' world centroid; several MORE updates do NOT re-centre (the
//!   `Local<bool>` latch holds) — proving the one-shot framing won't fight the sibling
//!   pan-nav slice (GTW-250).
//! - AC4 (`clamp_camera_to_bounds`): a camera placed FAR outside the battlefield bounds,
//!   with a known orthographic half-viewport + primary window, is pulled back inside (the
//!   `clamp_camera` relation holds end-to-end through the real system).
//! - GTW-271 AC6 (`clamp_camera_to_bounds` + viewport-aware fallback): a camera with a
//!   DEGENERATE orthographic `area` (forcing the pre-`camera_system` fallback) + an explicit
//!   sub-rect `Camera.viewport` is clamped using the SMALLER viewport-derived half-extent
//!   (the viewport physical size / window scale factor), NOT the full window — so reverting
//!   the `Some(viewport)` fallback branch to `window.size()` flips the asserted result.
//!
//! These prove the camera LOGIC headless; the actual on-screen centring is the user's
//! eyeball (AC5, post-gate QA). The systems are exercised on their REAL registration
//! shape (battle-gated `Update`), not a copy. Every `app.world_mut()` / camera mutation is
//! in a TEST BODY — the accepted headless idiom (`bevy-traps.md` #7 carve-out (a)). No
//! function here takes `&mut World`/`&World`.

use bevy::{
    MinimalPlugins,
    app::{App, Update},
    camera::{OrthographicProjection, Projection, Viewport},
    ecs::schedule::SystemCondition,
    math::{Rect, UVec2, Vec2},
    prelude::{Camera, Camera2d, IntoScheduleConfigs, Transform, With, resource_exists},
    window::{PrimaryWindow, Window, WindowResolution},
};
use gdtf_battle_presenter::{
    WorldCamera, camera_focus, cell_to_world, clamp_camera, clamp_camera_to_bounds,
    frame_camera_on_units,
};
use gdtf_battle_sim::{BattleInProgress, Cell, CellLevel, Faction, Level, PlayerFaction, Position};

/// The player gang for the AC3 fixture.
const PLAYER_GANG: u8 = 0;
/// An enemy gang the framing must IGNORE (only the player's gangers frame the camera).
const ENEMY_GANG: u8 = 1;

/// The known cells the two player gangers occupy in AC3 (ground storey).
const PLAYER_CELLS: [(i32, i32); 2] = [(10, 12), (20, 24)];
/// The enemy ganger's cell (off in a corner, to prove it does not move the centroid).
const ENEMY_CELL: (i32, i32) = (55, 3);

/// A synthetic viewport size (physical px) for the AC4 clamp test.
const VIEWPORT: Vec2 = Vec2::new(640.0, 480.0);

/// Builds a `Position` on the ground storey from a `(x, y)` cell.
fn ground_position(x: i32, y: i32) -> Position {
    Position::new(CellLevel::new(Cell::new(x, y), Level::new(0)))
}

/// The world-space centroid the AC3 framing must snap the camera to: the mean of the two
/// player gangers' `cell_to_world` ground positions (the SAME projection the systems use).
fn expected_player_focus() -> Vec2 {
    let centers = PLAYER_CELLS.into_iter().map(|(x, y)| {
        let world = cell_to_world(Cell::new(x, y), Level::new(0));
        Vec2::new(world.x, world.y)
    });
    camera_focus(centers).unwrap_or(Vec2::ZERO)
}

/// Reads the single `WorldCamera`'s translation `xy`.
fn camera_xy(app: &mut App) -> Vec2 {
    let world = app.world_mut();
    let mut query = world.query_filtered::<&Transform, With<WorldCamera>>();
    let mut found = Vec2::ZERO;
    for transform in query.iter(world) {
        found = Vec2::new(transform.translation.x, transform.translation.y);
    }
    found
}

// ---------------------------------------------------------------------------------
// AC3 — frame-on-units centres on the player centroid ONCE; the latch holds.
// ---------------------------------------------------------------------------------

/// AC3 — one update centres the `WorldCamera` on the player gangers' world centroid, and
/// further updates do NOT re-centre (the one-shot latch holds), so the framing won't fight
/// the later pan nav.
#[test]
fn frame_on_units_centres_once_and_latches() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins).add_systems(
        Update,
        frame_camera_on_units
            .run_if(resource_exists::<BattleInProgress>.and(resource_exists::<PlayerFaction>)),
    );

    // Battle-scoped witnesses the system gates on.
    app.world_mut().insert_resource(BattleInProgress);
    app.world_mut()
        .insert_resource(PlayerFaction::new(Faction::new(PLAYER_GANG)));

    // The camera starts at the origin (the static spawn never moves it).
    app.world_mut()
        .spawn((Camera2d, WorldCamera, Transform::default()));

    // Two player gangers at known cells + one enemy the framing must ignore.
    for (x, y) in PLAYER_CELLS {
        app.world_mut()
            .spawn((Faction::new(PLAYER_GANG), ground_position(x, y)));
    }
    app.world_mut().spawn((
        Faction::new(ENEMY_GANG),
        ground_position(ENEMY_CELL.0, ENEMY_CELL.1),
    ));

    // One update frames the camera on the player centroid.
    app.update();
    let focus = expected_player_focus();
    let after_first = camera_xy(&mut app);
    assert_eq!(
        after_first, focus,
        "the camera must centre on the PLAYER gangers' world centroid (enemy ignored)",
    );

    // The framing must NOT be the origin — i.e. it actually moved (the player cells are
    // off-origin), so a passing assertion above is real centring, not a no-op.
    assert_ne!(
        after_first,
        Vec2::ZERO,
        "the player centroid is off-origin, so the camera must have moved from spawn",
    );

    // Several MORE updates: the latch holds, so a later writer (pan nav) is not fought —
    // we simulate one by nudging the camera and confirming the framing leaves it alone.
    nudge_camera(&mut app, Vec2::new(999.0, -999.0));
    for _ in 0..3 {
        app.update();
    }
    let after_nudge = camera_xy(&mut app);
    assert_eq!(
        after_nudge,
        Vec2::new(999.0, -999.0),
        "after the one-shot framing, later updates must NOT re-centre the camera (latch holds)",
    );
}

/// Sets the `WorldCamera` translation `xy` to `to` in a TEST BODY (simulating a later
/// camera writer such as the pan nav).
fn nudge_camera(app: &mut App, to: Vec2) {
    let world = app.world_mut();
    let mut query = world.query_filtered::<&mut Transform, With<WorldCamera>>();
    for mut transform in query.iter_mut(world) {
        transform.translation.x = to.x;
        transform.translation.y = to.y;
    }
}

// ---------------------------------------------------------------------------------
// AC4 — the clamp pulls an out-of-bounds camera back inside the battlefield.
// ---------------------------------------------------------------------------------

/// AC4 — a camera placed FAR outside the battlefield bounds, with a known orthographic
/// half-viewport + primary window, is pulled back inside by `clamp_camera_to_bounds`.
#[test]
fn clamp_pulls_out_of_bounds_camera_back_inside() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins).add_systems(
        Update,
        clamp_camera_to_bounds
            .run_if(resource_exists::<BattleInProgress>.and(resource_exists::<PlayerFaction>)),
    );

    app.world_mut().insert_resource(BattleInProgress);
    app.world_mut()
        .insert_resource(PlayerFaction::new(Faction::new(PLAYER_GANG)));

    // A primary window the clamp reads for the viewport-size fallback.
    app.world_mut().spawn((
        Window {
            resolution: WindowResolution::new(VIEWPORT.x as u32, VIEWPORT.y as u32),
            ..Default::default()
        },
        PrimaryWindow,
    ));

    // A WorldCamera with a computed orthographic `area` (the synthetic-camera recipe:
    // `projection.update(w, h)` sets `area` to the viewport-sized world rect, as bevy's
    // `camera_system` would). Place it FAR outside the battlefield (negative corner).
    let mut projection = Projection::Orthographic(OrthographicProjection::default_2d());
    projection.update(VIEWPORT.x, VIEWPORT.y);
    let far_outside = Vec2::new(-100_000.0, 100_000.0);
    app.world_mut().spawn((
        Camera2d,
        WorldCamera,
        projection,
        Transform::from_xyz(far_outside.x, far_outside.y, 0.0),
    ));

    // Sanity: the camera starts far out of bounds.
    let before = camera_xy(&mut app);
    assert_eq!(
        before, far_outside,
        "precondition: the camera starts far out of bounds"
    );

    app.update();

    // The clamp must pull the camera inside the battlefield's world bounds. The bounds are
    // the 60x60 ground extent projected through `cell_to_world`; assert the relation (the
    // viewport stays within the bounds) rather than pinning a magnitude.
    let after = camera_xy(&mut app);
    assert_ne!(
        after, far_outside,
        "the clamp must MOVE a far-out-of-bounds camera",
    );

    // The battlefield bounds, recomputed the same way the system does, with the same
    // half-viewport (area.half_size() == VIEWPORT/2 after `update`).
    let bounds = battlefield_bounds();
    let half = VIEWPORT * 0.5;
    // The viewport [after - half, after + half] must sit within [min, max] on each axis
    // (the battlefield is far larger than this small viewport, so the clamp is the
    // boundary case, not the centre-when-smaller case).
    assert!(
        after.x - half.x >= bounds.0.x - f32::EPSILON
            && after.x + half.x <= bounds.1.x + f32::EPSILON,
        "after the clamp the viewport must stay within the battlefield x bounds",
    );
    assert!(
        after.y - half.y >= bounds.0.y - f32::EPSILON
            && after.y + half.y <= bounds.1.y + f32::EPSILON,
        "after the clamp the viewport must stay within the battlefield y bounds",
    );
}

/// The battlefield ground-plane world bounds `(min, max)`, projected the SAME way the
/// system does (the four corner cells of the 60x60 ground extent through `cell_to_world`).
/// Kept here in the test so AC4 asserts a RELATION, not a pinned magnitude.
fn battlefield_bounds() -> (Vec2, Vec2) {
    let w = i32::try_from(gdtf_battle_sim::GRID_WIDTH).unwrap_or(i32::MAX);
    let h = i32::try_from(gdtf_battle_sim::GRID_HEIGHT).unwrap_or(i32::MAX);
    let corners = [
        cell_to_world(Cell::new(0, 0), Level::new(0)),
        cell_to_world(Cell::new(w, 0), Level::new(0)),
        cell_to_world(Cell::new(0, h), Level::new(0)),
        cell_to_world(Cell::new(w, h), Level::new(0)),
    ];
    let mut min = Vec2::new(corners[0].x, corners[0].y);
    let mut max = min;
    for c in corners {
        let p = Vec2::new(c.x, c.y);
        min = min.min(p);
        max = max.max(p);
    }
    (min, max)
}

// ---------------------------------------------------------------------------------
// GTW-271 AC6 — the pre-`camera_system` fallback half-extent is VIEWPORT-aware.
// ---------------------------------------------------------------------------------

/// The AC6 window's physical width (the FULL surface — what the OLD full-window fallback
/// would use for the half-extent).
const AC6_WIN_W: u32 = 1600;
/// The AC6 window's physical height.
const AC6_WIN_H: u32 = 1200;
/// The AC6 window's scale factor — `> 1.0` so the physical→logical division in the fallback
/// is actually exercised (a revert to bare `physical_size`, dropping `/ scale_factor`, also
/// flips the asserted half-extent, not only a revert to the full window).
const AC6_SCALE: f32 = 2.0;
/// The AC6 map sub-rect's physical size — SMALLER than the window, so its derived half-extent
/// is smaller than the full-window fallback's and the two clamp results differ.
const AC6_VIEWPORT_PHYS: UVec2 = UVec2::new(800, 600);

/// GTW-271 AC6 — `clamp_camera_to_bounds` clamps using the VIEWPORT-derived half-extent (not
/// the full window) when the orthographic `area` is not yet computed (the pre-`camera_system`
/// fallback) and a sub-rect `Camera.viewport` is set.
///
/// Pin-discriminating: the camera starts far out of bounds with a DEGENERATE orthographic
/// `area` (half-size 0 → the steady-state `area_half > 0` branch is skipped and the fallback
/// runs) and an explicit sub-rect `Camera.viewport`. The assertion compares the clamped
/// position against `clamp_camera(.., viewport_half, ..)` (the SMALLER, viewport-derived
/// half-extent = viewport physical size / scale factor * 0.5) AND asserts it is NOT
/// `clamp_camera(.., window_half, ..)` (the larger, full-window half-extent the OLD fallback
/// produced). Reverting the `Some(viewport) => ..` fallback branch to `window.size()` makes
/// the system use `window_half`, flipping BOTH assertions. The bounds + both half-extents are
/// derived from `battlefield_bounds` + the (window, scale, viewport) sizes, so no scene
/// magnitude is pinned — only the viewport-vs-window RELATION.
#[test]
fn clamp_uses_the_viewport_half_extent_in_the_fallback() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins).add_systems(
        Update,
        clamp_camera_to_bounds
            .run_if(resource_exists::<BattleInProgress>.and(resource_exists::<PlayerFaction>)),
    );

    app.world_mut().insert_resource(BattleInProgress);
    app.world_mut()
        .insert_resource(PlayerFaction::new(Faction::new(PLAYER_GANG)));

    // A primary window at a KNOWN physical size + scale factor (> 1.0), so the fallback's
    // physical→logical conversion (`physical_size / scale_factor`) is exercised.
    let mut window = Window::default();
    window
        .resolution
        .set_physical_resolution(AC6_WIN_W, AC6_WIN_H);
    window.resolution.set_scale_factor(AC6_SCALE);
    app.world_mut().spawn((window, PrimaryWindow));

    // A WorldCamera whose orthographic `area` is DEGENERATE (half-size 0), so the steady-state
    // `area_half > 0` branch is skipped under MinimalPlugins (no `camera_system` to compute the
    // area) and the GTW-271 fallback runs. `scale = 1.0` keeps the half-extent arithmetic clean.
    let projection = Projection::Orthographic(OrthographicProjection {
        scale: 1.0,
        area: Rect::from_corners(Vec2::ZERO, Vec2::ZERO),
        ..OrthographicProjection::default_2d()
    });
    // The sub-rect map viewport (physical px) the app would set in AC1. Origin is irrelevant to
    // the half-extent (only the SIZE feeds it).
    let camera = Camera {
        viewport: Some(Viewport {
            physical_position: UVec2::ZERO,
            physical_size:     AC6_VIEWPORT_PHYS,
            depth:             0.0..1.0,
        }),
        ..Default::default()
    };
    let far_outside = Vec2::new(-100_000.0, -100_000.0);
    app.world_mut().spawn((
        Camera2d,
        WorldCamera,
        camera,
        projection,
        Transform::from_xyz(far_outside.x, far_outside.y, 0.0),
    ));

    app.update();

    let after = camera_xy(&mut app);
    let (min, max) = battlefield_bounds();

    // The half-extents the two fallback branches produce (scale = 1.0): the viewport branch
    // divides by the scale factor, the full-window branch uses the logical window size.
    let viewport_half = AC6_VIEWPORT_PHYS.as_vec2() / AC6_SCALE * 0.5;
    let window_logical = Vec2::new(AC6_WIN_W as f32, AC6_WIN_H as f32) / AC6_SCALE;
    let window_half = window_logical * 0.5;

    // Sanity: the viewport half-extent really is SMALLER, so the two clamp results differ.
    assert!(
        viewport_half.x < window_half.x && viewport_half.y < window_half.y,
        "fixture invariant: the sub-rect viewport must derive a smaller half-extent than the window",
    );

    // The clamp must use the VIEWPORT-derived half-extent.
    let expected_viewport = clamp_camera(far_outside, viewport_half, min, max);
    assert_eq!(
        after, expected_viewport,
        "the fallback clamp must use the VIEWPORT-derived half-extent (physical size / scale factor)",
    );

    // And NOT the full-window half-extent (what the OLD fallback / a revert produces) — this is
    // the AC6 pin: reverting `Some(viewport) => ..` to `window.size()` flips this assertion.
    let reverted_window = clamp_camera(far_outside, window_half, min, max);
    assert_ne!(
        after, reverted_window,
        "the fallback must NOT clamp with the full-window half-extent once a sub-rect viewport is set",
    );
}
