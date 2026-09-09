use bevy::{prelude::*, scene::ScenePlugin, window::PrimaryWindow};
use cobalt_test_utils::unwatched_asset_plugin;
use gdtf_battle_presenter::{WorldCamera, spawn_world_camera};

use super::set_world_viewport;
use crate::states::running::game::battlescape::bottom_bar::BottomBarRoot;

const BAR_HEIGHT: f32 = 180.0;
const BAR_HEIGHT_PX: u32 = 180;
const WIN_W: u32 = 1280;
const WIN_H: u32 = 720;

fn computed(size: Vec2) -> ComputedNode {
    ComputedNode { size, ..default() }
}

fn spawn_window(app: &mut App) {
    let mut window = Window::default();
    window.resolution.set_physical_resolution(WIN_W, WIN_H);
    window.resolution.set_scale_factor(1.0);
    app.world_mut().spawn((window, PrimaryWindow));
}

fn world_viewport(app: &mut App) -> Option<bevy::camera::Viewport> {
    let mut cameras = app
        .world_mut()
        .query_filtered::<&Camera, With<WorldCamera>>();
    cameras
        .iter(app.world())
        .next()
        .and_then(|camera| camera.viewport.clone())
}

#[test]
fn set_world_viewport_insets_the_map_by_the_bottom_bar_only() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, unwatched_asset_plugin(), ScenePlugin));
    spawn_window(&mut app);

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

    assert_eq!(
        viewport.physical_position,
        UVec2::ZERO,
        "the viewport origin is the window top-left (no left/top inset)",
    );
    assert_eq!(
        viewport.physical_size,
        UVec2::new(WIN_W, WIN_H - BAR_HEIGHT_PX),
        "the viewport is full width, full height minus the bottom-bar height",
    );
}

#[test]
fn set_world_viewport_full_window_when_no_bottom_bar() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, unwatched_asset_plugin(), ScenePlugin));
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
