//! Highlight draw: hover lands a cell highlight on occupied or blocking cells.
use bevy::{
    app::App,
    camera::{
        Camera, ComputedCameraValues, OrthographicProjection, Projection, RenderTargetInfo,
        primitives::Frustum,
    },
    math::{Vec2, Vec3},
    prelude::*,
    transform::components::GlobalTransform,
    window::{PrimaryWindow, Window, WindowResolution},
};
use gdtf_battle_input::{GdtfBattleInputPlugin, InspectTarget, world_to_cell};
use gdtf_battle_presenter::{
    ActiveLevel, HoverHighlight, TopDownRendererPlugin, WorldCamera, cell_to_world,
};
use gdtf_battle_sim::{
    occupancy::TerrainKind,
    prelude::{BattleInProgress, Cell, CellLevel, Level, OccupancyGrid},
    visibility::SquadVisibility,
};

const TARGET_SIZE: Vec2 = Vec2::new(1280.0, 720.0);

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

fn e2e_app(active_level: Level) -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
    ))
    .add_plugins(GdtfBattleInputPlugin)
    .add_plugins(TopDownRendererPlugin);
    app.world_mut()
        .insert_resource(ActiveLevel::new(active_level));
    app.world_mut().insert_resource(BattleInProgress);
    app.world_mut().insert_resource(OccupancyGrid::new());

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

fn set_cursor(app: &mut App, position: Option<Vec2>) {
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

fn highlight_state(app: &mut App) -> Option<(Vec3, Visibility)> {
    let mut q = app
        .world_mut()
        .query_filtered::<(&Transform, &Visibility), With<HoverHighlight>>();
    let mut iter = q.iter(app.world());
    iter.next().map(|(t, v)| (t.translation, *v))
}

fn highlight_count(app: &mut App) -> usize {
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, With<HoverHighlight>>();
    q.iter(app.world()).count()
}

#[test]
fn end_to_end_highlight_lands_at_the_hovered_cell() {
    let level = Level::new(0);
    let mut app = e2e_app(level);

    let cursor = TARGET_SIZE * 0.5 + Vec2::new(40.0, 32.0);
    set_cursor(&mut app, Some(cursor));
    app.update();

    let cell = hovered(&app);
    assert!(cell.is_some(), "the in-grid cursor must resolve a cell");
    let Some(cell) = cell else { return };

    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.set_terrain(cell, TerrainKind::Cover);
    }
    app.update();
    app.update();

    assert_eq!(
        highlight_count(&mut app),
        1,
        "exactly one highlight sprite after the real picker path",
    );
    assert_eq!(
        highlight_state(&mut app),
        Some((cell_to_world(cell.cell(), level), Visibility::Visible,)),
        "the highlight must be Visible at cell_to_world(the hovered cell)",
    );

    let wrong = cell_to_world(Cell::new(cell.x + 3, cell.y + 3), level);
    assert_ne!(
        highlight_state(&mut app).map(|(t, _)| t),
        Some(wrong),
        "the highlight must be at the hovered cell, not a neighbouring one",
    );

    let cursor_off = TARGET_SIZE * 0.5 - Vec2::new(64.0, 0.0);
    set_cursor(&mut app, Some(cursor_off));
    let world_off = {
        let mut q = app
            .world_mut()
            .query_filtered::<(&Camera, &GlobalTransform), With<WorldCamera>>();
        q.iter(app.world())
            .next()
            .and_then(|(cam, t)| cam.viewport_to_world_2d(t, cursor_off).ok())
    };
    let Some(world_off) = world_off else { return };
    assert_eq!(
        world_to_cell(world_off, level),
        None,
        "the chosen left-of-origin cursor must be off-grid (world {world_off:?})",
    );
    app.update();
    app.update();
    assert_eq!(
        hovered(&app),
        None,
        "the off-grid cursor clears InspectTarget"
    );
    assert_eq!(
        highlight_count(&mut app),
        1,
        "the highlight entity persists (hidden, not duplicated) off-grid",
    );
    assert_eq!(
        highlight_state(&mut app).map(|(_, v)| v),
        Some(Visibility::Hidden),
        "the highlight must hide when the cursor leaves the grid",
    );
}

#[test]
fn highlight_only_on_occupied_or_blocking_cells() {
    let level = Level::new(0);
    let mut app = e2e_app(level);

    let cursor = TARGET_SIZE * 0.5 + Vec2::new(40.0, 32.0);
    set_cursor(&mut app, Some(cursor));
    app.update();

    let cell = hovered(&app);
    assert!(cell.is_some(), "the in-grid cursor must resolve a cell");
    let Some(cell) = cell else { return };

    app.update();
    app.update();
    assert_ne!(
        highlight_state(&mut app).map(|(_, v)| v),
        Some(Visibility::Visible),
        "a bare-floor cell must NOT highlight — gangers and objects only",
    );
    assert_eq!(
        highlight_count(&mut app),
        0,
        "no highlight sprite is spawned while only bare floor is hovered",
    );

    let ganger = app.world_mut().spawn_empty().id();
    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.set_occupant(cell, Some(ganger));
    }
    let visible: bevy::platform::collections::HashSet<CellLevel> = core::iter::once(cell).collect();
    app.world_mut()
        .insert_resource(SquadVisibility::new(visible.clone(), visible));
    app.update();
    app.update();
    assert_eq!(
        highlight_state(&mut app),
        Some((cell_to_world(cell.cell(), level), Visibility::Visible,)),
        "a squad-VISIBLE cell holding a ganger (occupant) MUST highlight at that cell ",
    );

    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.set_occupant(cell, None);
        grid.set_terrain(cell, TerrainKind::Cover);
    }
    app.update();
    app.update();
    assert_eq!(
        highlight_state(&mut app).map(|(_, v)| v),
        Some(Visibility::Visible),
        "a blocking object / cover cell MUST highlight (OR-branch)",
    );
}
