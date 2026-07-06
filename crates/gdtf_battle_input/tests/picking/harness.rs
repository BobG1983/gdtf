//! Shared picking fixture: the synthetic camera / window picking app, the
//! cursor drive, and the hovered / unproject probes.

use bevy::{
    app::App,
    camera::{
        Camera, ComputedCameraValues, OrthographicProjection, Projection, RenderTargetInfo,
        primitives::Frustum,
    },
    math::Vec2,
    prelude::*,
    transform::components::GlobalTransform,
    window::{PrimaryWindow, Window, WindowResolution},
};
use gdtf_battle_input::{GdtfBattleInputPlugin, InspectTarget};
use gdtf_battle_presenter::{ActiveLevel, ViewMode, WorldCamera};
use gdtf_battle_sim::prelude::{BattleInProgress, CellLevel, Level};

/// The synthetic window/camera render-target size (physical px), large enough that a
/// cursor near its centre unprojects to an in-grid cell.
pub(crate) const TARGET_SIZE: Vec2 = Vec2::new(1280.0, 720.0);

// ---------------------------------------------------------------------------------
// AC2-AC4 — focused picking/highlight harness with a synthesized camera + cursor.
// ---------------------------------------------------------------------------------

/// Builds a deterministic [`Camera`] whose `viewport_to_world_2d` succeeds WITHOUT a
/// render pipeline, mirroring bevy's own `viewport_to_world` unit test: set the
/// render-target info + viewport, run the projection's `update`, and store its
/// clip-from-view matrix in `computed`.
pub(crate) fn synthetic_camera() -> Camera {
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
pub(crate) fn picking_app(active_level: Level) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(GdtfBattleInputPlugin);
    // The presenter normally owns ActiveLevel (init in TopDownRendererPlugin); the
    // focused harness inserts it directly so picking has a level to band on.
    app.world_mut()
        .insert_resource(ActiveLevel::new(active_level));
    // GTW-521 — `dispatch_act_intents` also mutates the presenter-owned `ViewMode`.
    app.world_mut().insert_resource(ViewMode::default());
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
pub(crate) fn set_cursor(app: &mut App, position: Option<Vec2>) {
    let mut windows = app.world_mut().query::<&mut Window>();
    for mut window in windows.iter_mut(app.world_mut()) {
        window.set_cursor_position(position);
    }
}

/// Reads the current `InspectTarget` live hovered cell.
pub(crate) fn hovered(app: &App) -> Option<CellLevel> {
    app.world()
        .get_resource::<InspectTarget>()
        .and_then(InspectTarget::hovered)
}

/// The camera's world unprojection of `cursor` — what the picking system computes
/// internally. The test re-derives the expected cell from this with the documented
/// inverse, so it never hardcodes the world math.
pub(crate) fn unproject(app: &mut App, cursor: Vec2) -> Option<Vec2> {
    let mut q = app
        .world_mut()
        .query_filtered::<(&Camera, &GlobalTransform), With<WorldCamera>>();
    let (camera, transform) = q.iter(app.world()).next()?;
    camera.viewport_to_world_2d(transform, cursor).ok()
}
