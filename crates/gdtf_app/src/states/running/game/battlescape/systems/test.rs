//! GTW-271 / GTW-275 layout overhaul — `set_world_viewport` confines the world map to the
//! FULL window MINUS only the bottom-bar height.
//!
//! Headless, pin-discriminating: with a known window size, the REAL bottom-bar root marker
//! ([`BottomBarRoot`] — the marker the real `spawn_bottom_bar` attaches) carrying a known
//! `ComputedNode` height (the layout engine — `ui_layout_system` under the full `UiPlugin` — is
//! the one stubbed external; this test owns the viewport-inset math, not bevy's layouter), and
//! the REAL `spawn_world_camera`, the `set_world_viewport` system is driven and its
//! `WorldCamera` viewport asserted to be the window MINUS the bottom-bar height (full width, no
//! side / top inset). It would FAIL if a side/top inset were reintroduced, the viewport write
//! removed, or the bottom-bar marker dropped from the inset query.

use bevy::{asset::AssetPlugin, prelude::*, scene::ScenePlugin, window::PrimaryWindow};
use gdtf_battle_presenter::{WorldCamera, spawn_world_camera};

use super::set_world_viewport;
use crate::states::running::game::battlescape::bottom_bar::BottomBarRoot;

/// The bottom-bar height (physical px, f32 for `ComputedNode`) — the BOTTOM inset.
const BAR_HEIGHT: f32 = 180.0;
/// The bottom-bar height rounded to physical px (the expected `u32` inset — kept as a separate
/// literal so the test asserts without an `f32 as u32` cast).
const BAR_HEIGHT_PX: u32 = 180;
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

/// GTW-275 layout overhaul — `set_world_viewport` sets the `WorldCamera`'s `Camera.viewport` to
/// the FULL window inset BOTTOM by the bottom-bar height ONLY (no left / right / top inset).
///
/// Pin-discriminating: the asserted `physical_position` / `physical_size` are derived from the
/// injected bottom-bar `ComputedNode` height, so reintroducing a side/top inset, removing the
/// viewport write, or dropping the bottom-bar marker from the query would change the numbers and
/// fail.
#[test]
fn set_world_viewport_insets_the_map_by_the_bottom_bar_only() {
    let mut app = App::new();
    // GTW-322 — `spawn_world_camera` now authors its camera via `bsn!` / `spawn_scene`, which
    // resolves in the `SpawnScene` schedule and PANICS without the scene resources, so this
    // headless harness adds `AssetPlugin` + `ScenePlugin` directly (the widget-builder slice
    // precedent). The camera has no asset deps, so the scene materializes on the first
    // `update`; the second `update` then runs `set_world_viewport` against the live camera.
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    spawn_window(&mut app);

    // The REAL world camera (presenter `spawn_world_camera`) + the REAL bottom-bar root marker
    // with an injected layout height (full width — only the height insets the map).
    app.add_systems(Startup, spawn_world_camera);
    app.world_mut()
        .spawn((BottomBarRoot, computed(Vec2::new(1280.0, BAR_HEIGHT))));

    app.add_systems(Update, set_world_viewport);
    app.update();
    app.update();

    let viewport = world_viewport(&mut app);
    assert!(
        viewport.is_some(),
        "the world camera must have a viewport set after set_world_viewport runs",
    );
    let Some(viewport) = viewport else { return };

    // The map origin is the window top-left — NO left / top inset (the corner panels are
    // overlays, items 2 / 3 / 4).
    assert_eq!(
        viewport.physical_position,
        UVec2::ZERO,
        "the viewport origin is the window top-left (no left/top inset)",
    );
    // The map is FULL width, full height minus only the bottom-bar height (item 1 / 4).
    assert_eq!(
        viewport.physical_size,
        UVec2::new(WIN_W, WIN_H - BAR_HEIGHT_PX),
        "the viewport is full width, full height minus the bottom-bar height",
    );
}

/// GTW-275 layout overhaul — with NO bottom bar spawned (before the bar spawns), the system
/// writes a FULL-window viewport (the bottom inset is 0) rather than failing to set one — the
/// "tolerate the bar not existing → 0" allowance, and the saturating math never wraps.
#[test]
fn set_world_viewport_full_window_when_no_bottom_bar() {
    let mut app = App::new();
    // GTW-322 — see the sibling test: `spawn_world_camera`'s `bsn!` scene needs the scene
    // resources + a settling `update` before `set_world_viewport` can read the live camera.
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    spawn_window(&mut app);

    app.add_systems(Startup, spawn_world_camera);
    app.add_systems(Update, set_world_viewport);
    app.update();
    app.update();

    let viewport = world_viewport(&mut app);
    assert!(
        viewport.is_some(),
        "the world camera must have a (full-window) viewport set even with no bottom bar",
    );
    let Some(viewport) = viewport else { return };
    assert_eq!(
        viewport.physical_position,
        UVec2::ZERO,
        "with no bottom bar the viewport origin is the window top-left",
    );
    assert_eq!(
        viewport.physical_size,
        UVec2::new(WIN_W, WIN_H),
        "with no bottom bar the viewport fills the whole window",
    );
}
