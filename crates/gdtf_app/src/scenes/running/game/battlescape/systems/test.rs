//! GTW-271 AC1 — `set_world_viewport` confines the world map to the central sub-rect.
//!
//! Headless, pin-discriminating: with a known window size, the REAL panel root markers
//! ([`StatusPanelRoot`] / [`ActionBarRoot`] — the markers the real `spawn_status_panel` /
//! `spawn_action_bar` attach) carrying known `ComputedNode` sizes (the layout engine —
//! `ui_layout_system` under the full `UiPlugin` — is the one stubbed external; this test owns
//! the viewport-inset math, not bevy's layouter), and the REAL `spawn_world_camera`, the
//! `set_world_viewport` system is driven and its `WorldCamera` viewport asserted to be the
//! window MINUS the panel margins. It would FAIL if the inset axes / margins were reverted, the
//! viewport write removed, or a panel marker dropped from the inset query.

use bevy::{prelude::*, window::PrimaryWindow};
use gdtf_battle_presenter::{WorldCamera, spawn_world_camera};

use super::set_world_viewport;
use crate::scenes::running::game::battlescape::{
    action_bar::ActionBarRoot, status_panel::StatusPanelRoot,
};

/// The status-panel width (physical px, f32 for `ComputedNode`) — the LEFT inset.
const STATUS_WIDTH: f32 = 220.0;
/// The status-panel width rounded to physical px (the expected `u32` inset — kept as a separate
/// literal so the test asserts without an `f32 as u32` cast).
const STATUS_WIDTH_PX: u32 = 220;
/// The action-bar height (physical px, f32 for `ComputedNode`) — the BOTTOM inset.
const BAR_HEIGHT: f32 = 90.0;
/// The action-bar height rounded to physical px (the expected `u32` inset).
const BAR_HEIGHT_PX: u32 = 90;
/// The test window's physical width.
const WIN_W: u32 = 1280;
/// The test window's physical height.
const WIN_H: u32 = 720;

/// Builds a `ComputedNode` with a given physical size (the layout-engine stand-in).
fn computed(size: Vec2) -> ComputedNode {
    ComputedNode { size, ..default() }
}

/// Spawns a known-size primary window at scale factor 1.0 (so physical = logical, unambiguous).
fn spawn_window(app: &mut App) {
    let mut window = Window::default();
    window.resolution.set_physical_resolution(WIN_W, WIN_H);
    window.resolution.set_scale_factor(1.0);
    app.world_mut().spawn((window, PrimaryWindow));
}

/// The world camera's current viewport, if any.
fn world_viewport(app: &mut App) -> Option<bevy::camera::Viewport> {
    let mut cameras = app
        .world_mut()
        .query_filtered::<&Camera, With<WorldCamera>>();
    cameras
        .iter(app.world())
        .next()
        .and_then(|camera| camera.viewport.clone())
}

/// GTW-271 AC1 — `set_world_viewport` sets the `WorldCamera`'s `Camera.viewport` to the window
/// inset LEFT by the status-panel width and BOTTOM by the action-bar height (TOP/RIGHT minimal).
///
/// Pin-discriminating: the asserted `physical_position` / `physical_size` are derived from the
/// injected `ComputedNode` sizes, so reverting the inset (e.g. dropping a margin, or insetting
/// the wrong axis), removing the viewport write, or dropping a panel marker from the query would
/// change the numbers and fail.
#[test]
fn set_world_viewport_insets_the_map_by_the_panel_margins() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    spawn_window(&mut app);

    // The REAL world camera (presenter `spawn_world_camera`) + the REAL panel root markers with
    // injected layout sizes (the status panel top-left, the action-bar bottom).
    app.add_systems(Startup, spawn_world_camera);
    app.world_mut()
        .spawn((StatusPanelRoot, computed(Vec2::new(STATUS_WIDTH, 400.0))));
    app.world_mut()
        .spawn((ActionBarRoot, computed(Vec2::new(600.0, BAR_HEIGHT))));

    app.add_systems(Update, set_world_viewport);
    app.update();

    let viewport = world_viewport(&mut app);
    assert!(
        viewport.is_some(),
        "the world camera must have a viewport set after set_world_viewport runs",
    );
    let Some(viewport) = viewport else { return };

    // LEFT inset = status width, BOTTOM inset = bar height, TOP/RIGHT = 0.
    assert_eq!(
        viewport.physical_position,
        UVec2::new(STATUS_WIDTH_PX, 0),
        "the viewport origin must inset LEFT by the status width and TOP by 0",
    );
    assert_eq!(
        viewport.physical_size,
        UVec2::new(WIN_W - STATUS_WIDTH_PX, WIN_H - BAR_HEIGHT_PX),
        "the viewport size must be the window minus the left + bottom margins",
    );
}

/// GTW-271 AC1 — with NO panels spawned (the hover panel never exists yet, and before the
/// panels spawn), `set_world_viewport` writes a FULL-window viewport (every inset 0) rather than
/// failing to set one — the "tolerate a panel not existing → 0" allowance, and the saturating
/// math never wraps.
#[test]
fn set_world_viewport_full_window_when_no_panels() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    spawn_window(&mut app);

    app.add_systems(Startup, spawn_world_camera);
    app.add_systems(Update, set_world_viewport);
    app.update();

    let viewport = world_viewport(&mut app);
    assert!(
        viewport.is_some(),
        "the world camera must have a (full-window) viewport set even with no panels",
    );
    let Some(viewport) = viewport else { return };
    assert_eq!(
        viewport.physical_position,
        UVec2::ZERO,
        "with no panels the viewport origin is the window top-left",
    );
    assert_eq!(
        viewport.physical_size,
        UVec2::new(WIN_W, WIN_H),
        "with no panels the viewport fills the whole window",
    );
}
