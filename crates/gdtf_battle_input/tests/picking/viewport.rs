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
    acts::MoveRequested,
    battle::PlayerFaction,
    prelude::{Faction, Level, OccupancyGrid},
    tuning::CombatTuning,
    vertical::VerticalLinkGraph,
};
use gdtf_test_utils::{MessageProbePlugin, clear_mouse, press_left, probed};

use super::harness::*;


const PLAYER_FACTION: Faction = Faction::new(0);

const VIEWPORT_RECT: URect = URect {
    min: UVec2::new(320, 180),
    max: UVec2::new(960, 540),
};

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

fn move_path_app(active_level: Level) -> App {
    let mut app = picking_app(active_level);
    set_world_viewport(&mut app, VIEWPORT_RECT);
    app.world_mut().insert_resource(OccupancyGrid::default());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut()
        .insert_resource(PlayerFaction::new(PLAYER_FACTION));
    app.world_mut()
        .insert_resource(VerticalLinkGraph::default());
    app.world_mut()
        .insert_resource(ButtonInput::<MouseButton>::default());

    let ganger = app.world_mut().spawn(PLAYER_FACTION).id();
    app.world_mut()
        .insert_resource(gdtf_battle_input::SelectedShooter::new(ganger));

    app.add_plugins(MessageProbePlugin::<MoveRequested>::default());
    app
}

fn move_requests(app: &App) -> Vec<MoveRequested> {
    probed::<MoveRequested>(app)
}

#[test]
fn two_click_inside_the_viewport_resolves_a_cell_and_moves() {
    let level = Level::new(0);
    let mut app = move_path_app(level);

    let viewport_centre = VIEWPORT_RECT.center().as_vec2();
    let cursor = viewport_centre + Vec2::new(20.0, 16.0);

    set_cursor(&mut app, Some(cursor));
    app.update();

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

    press_left(&mut app);
    app.update();
    assert!(
        move_requests(&app).is_empty(),
        "click-1 on an in-viewport empty cell SETS the target — no MoveRequested yet (GTW-356)",
    );

    clear_mouse(&mut app);
    press_left(&mut app);
    app.update();
    assert_eq!(
        move_requests(&app).len(),
        1,
        "click-2 on the same in-viewport cell must commit exactly one MoveRequested (GTW-356)",
    );
}

#[test]
fn click_in_the_bottom_margin_resolves_none_and_does_not_move() {
    let level = Level::new(0);
    let mut app = move_path_app(level);

    let centre_x = VIEWPORT_RECT.center().as_vec2().x;
    let margin_y = VIEWPORT_RECT.max.y as f32 + 4.0;
    let cursor = Vec2::new(centre_x, margin_y);

    let world = unproject(&mut app, cursor);
    let Some(world) = world else {
        unreachable!("the synthetic camera must unproject the margin cursor");
    };
    assert!(
        world_to_cell(world, level).is_some(),
        "the margin cursor's extrapolated world point must floor to an IN-GRID cell \
         (world {world:?}) — otherwise the test would pass for the wrong reason",
    );

    set_cursor(&mut app, Some(cursor));
    app.update();
    assert_eq!(
        hovered(&app),
        None,
        "a cursor in the bottom margin (outside the viewport rect) must resolve InspectTarget to \
         None — even though its extrapolated cell is in-grid (GTW-286 gate)",
    );

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

#[test]
fn two_click_over_a_panel_is_absorbed_and_does_not_move() {
    let level = Level::new(0);
    let mut app = move_path_app(level);

    let viewport_centre = VIEWPORT_RECT.center().as_vec2();
    let cursor = viewport_centre + Vec2::new(20.0, 16.0);

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

    spawn_ui_panel(&mut app, cursor, Vec2::new(200.0, 120.0));

    set_cursor(&mut app, Some(cursor));
    app.update();
    assert_eq!(
        hovered(&app),
        None,
        "a cursor over a HUD panel must resolve InspectTarget to None — the UI absorbs the \
         click (GTW-380), even though the cell underneath is in-grid + in-viewport",
    );

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
