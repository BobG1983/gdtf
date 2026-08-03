use bevy::{
    camera::{
        Camera, ComputedCameraValues, OrthographicProjection, Projection, RenderTargetInfo,
        primitives::Frustum,
    },
    math::Vec2,
    prelude::*,
    transform::components::GlobalTransform,
    window::{CursorMoved, PrimaryWindow, Window, WindowResolution},
};
use gdtf_battle_input::{ActivePointer, GamepadCursor, GdtfBattleInputPlugin, InspectTarget};
use gdtf_battle_presenter::{
    ActiveLevel, CellVisibility, HighlightRequest, ViewMode, WorldCamera, cell_to_world,
};
use gdtf_battle_sim::{
    occupancy::TerrainKind,
    prelude::{BattleInProgress, CellLevel, OccupancyGrid},
};
use gdtf_test_utils::{MessageProbe, MessageProbePlugin, probed};

use super::harness::*;


fn synthetic_camera() -> Camera {
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

fn picking_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(GdtfBattleInputPlugin);
    app.world_mut().insert_resource(ActiveLevel::new(LEVEL));
    app.world_mut().insert_resource(ViewMode::default());
    app.world_mut().insert_resource(BattleInProgress);
    app.world_mut().spawn((
        Camera2d,
        WorldCamera,
        synthetic_camera(),
        GlobalTransform::IDENTITY,
        Projection::Orthographic(OrthographicProjection::default_2d()),
        Frustum::default(),
    ));
    app.world_mut().spawn((
        Window {
            resolution: WindowResolution::new(TARGET_SIZE.x as u32, TARGET_SIZE.y as u32),
            ..default()
        },
        PrimaryWindow,
    ));
    app
}

fn set_os_cursor(app: &mut App, position: Option<Vec2>) {
    let mut windows = app.world_mut().query::<&mut Window>();
    for mut window in windows.iter_mut(app.world_mut()) {
        window.set_cursor_position(position);
    }
}

fn hovered(app: &App) -> Option<CellLevel> {
    app.world()
        .get_resource::<InspectTarget>()
        .and_then(InspectTarget::hovered)
}

fn unproject(app: &mut App, cursor: Vec2) -> Option<Vec2> {
    let mut q = app
        .world_mut()
        .query_filtered::<(&Camera, &GlobalTransform), With<WorldCamera>>();
    let (camera, transform) = q.iter(app.world()).next()?;
    camera.viewport_to_world_2d(transform, cursor).ok()
}

#[test]
fn picker_honors_active_pointer() {
    let mut app = picking_app();

    let gamepad_screen = TARGET_SIZE * 0.5 + Vec2::new(40.0, 32.0);
    let os_screen = TARGET_SIZE * 0.5 + Vec2::new(80.0, 64.0);
    set_os_cursor(&mut app, Some(os_screen));

    app.world_mut().insert_resource(ActivePointer::Gamepad);
    app.world_mut()
        .insert_resource(GamepadCursor::new(gamepad_screen));
    app.update();

    let gamepad_world = unproject(&mut app, gamepad_screen);
    let os_world = unproject(&mut app, os_screen);
    assert!(
        gamepad_world.is_some() && os_world.is_some(),
        "the synthetic camera must unproject both screen points",
    );
    let (Some(gamepad_world), Some(os_world)) = (gamepad_world, os_world) else {
        return;
    };
    let gamepad_cell = gdtf_battle_input::world_to_cell(gamepad_world, LEVEL);
    let os_cell = gdtf_battle_input::world_to_cell(os_world, LEVEL);
    assert!(
        gamepad_cell.is_some() && os_cell.is_some() && gamepad_cell != os_cell,
        "the two cursors must land on distinct in-grid cells for the arbitration to be \
         observable (gamepad {gamepad_cell:?}, os {os_cell:?})",
    );

    assert_eq!(
        hovered(&app),
        gamepad_cell,
        "in Gamepad mode the picker must project the GAMEPAD cursor, not the OS cursor",
    );

    app.world_mut().insert_resource(ActivePointer::Mouse);
    app.update();
    assert_eq!(
        hovered(&app),
        os_cell,
        "flipping to Mouse must revert the picker to the OS cursor",
    );
}

#[test]
fn highlight_follows_the_gamepad_cursor() {
    let mut app = picking_app();
    app.world_mut().insert_resource(OccupancyGrid::default());
    app.add_plugins(MessageProbePlugin::<HighlightRequest>::default());

    set_os_cursor(&mut app, None);
    let gamepad_screen = TARGET_SIZE * 0.5 + Vec2::new(40.0, 32.0);
    app.world_mut().insert_resource(ActivePointer::Gamepad);
    app.world_mut()
        .insert_resource(GamepadCursor::new(gamepad_screen));
    app.update();

    let cell = hovered(&app);
    assert!(
        cell.is_some(),
        "the gamepad cursor must resolve an in-grid cell"
    );
    let Some(resolved) = cell else { return };

    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.set_terrain(resolved, TerrainKind::Cover);
    }
    if let Some(mut probe) = app
        .world_mut()
        .get_resource_mut::<MessageProbe<HighlightRequest>>()
    {
        probe.clear();
    }
    app.update();

    let emitted = probed::<HighlightRequest>(&app);
    assert_eq!(
        emitted,
        vec![HighlightRequest::new(cell, CellVisibility::NotSquadVisible)],
        "the highlight request must follow the gamepad cursor's resolved cell (GTW-11: \
         NotSquadVisible — no fog seeded, fail-closed)",
    );
    if let Some(cell) = cell {
        let _ = cell_to_world(cell.cell(), LEVEL);
    }
}


#[test]
fn mouse_reclaims_the_pointer() {
    let mut app = picking_app();
    app.add_message::<CursorMoved>();
    app.world_mut().insert_resource(ActivePointer::Gamepad);
    app.update();
    assert_eq!(
        *app.world().resource::<ActivePointer>(),
        ActivePointer::Gamepad,
        "precondition: the pointer starts on Gamepad",
    );

    let window = {
        let mut q = app
            .world_mut()
            .query_filtered::<Entity, With<PrimaryWindow>>();
        q.iter(app.world()).next()
    };
    assert!(
        window.is_some(),
        "the picking app must have a primary window"
    );
    let Some(window) = window else {
        return;
    };

    app.world_mut()
        .resource_mut::<bevy::ecs::message::Messages<CursorMoved>>()
        .write(CursorMoved {
            window,
            position: Vec2::new(100.0, 100.0),
            delta: Some(Vec2::new(5.0, 5.0)),
        });
    app.update();

    assert_eq!(
        *app.world().resource::<ActivePointer>(),
        ActivePointer::Mouse,
        "a CursorMoved message must flip the active pointer back to Mouse (last-moved-wins)",
    );
}
