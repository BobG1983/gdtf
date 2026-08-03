use bevy::{
    app::{App, Last, Update},
    camera::{RenderTarget, visibility::Visibility},
    math::Vec2,
    picking::{
        hover::PickingInteraction,
        pointer::{Location, PointerId, PointerLocation},
    },
    prelude::*,
    transform::components::GlobalTransform,
    ui::{ComputedNode, UiGlobalTransform},
    window::{PrimaryWindow, Window, WindowRef, WindowResolution},
};
use gdtf_battle_input::{GdtfBattleInputPlugin, InspectTarget, world_to_cell};
use gdtf_battle_presenter::{ActiveLevel, ViewMode, WorldCamera};
use gdtf_battle_sim::{
    acts::MoveRequested,
    battle::PlayerFaction,
    prelude::{BattleInProgress, Faction, Level, OccupancyGrid},
    tuning::CombatTuning,
    vertical::VerticalLinkGraph,
};
use gdtf_test_utils::{GdtfUiTestAppBuilder, MessageProbePlugin, clear_mouse, press_left, probed};

use super::harness::{TARGET_SIZE, synthetic_camera};

const PLAYER_FACTION: Faction = Faction::new(0);

const PANEL_SIZE: Vec2 = Vec2::new(240.0, 140.0);

const PANEL_ORIGIN: Vec2 = Vec2::new(520.0, 290.0);

fn coexistence_app(level: Level) -> (App, Entity) {
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    app.add_plugins(GdtfBattleInputPlugin);

    app.world_mut().insert_resource(ActiveLevel::new(level));
    app.world_mut().insert_resource(ViewMode::default());
    app.world_mut().insert_resource(BattleInProgress);
    app.world_mut().insert_resource(OccupancyGrid::default());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut()
        .insert_resource(PlayerFaction::new(PLAYER_FACTION));
    app.world_mut()
        .insert_resource(VerticalLinkGraph::default());

    let ganger = app.world_mut().spawn(PLAYER_FACTION).id();
    app.world_mut()
        .insert_resource(gdtf_battle_input::SelectedShooter::new(ganger));

    app.world_mut().spawn((
        Window {
            resolution: WindowResolution::new(TARGET_SIZE.x as u32, TARGET_SIZE.y as u32),
            ..default()
        },
        PrimaryWindow,
    ));

    app.world_mut()
        .spawn((WorldCamera, synthetic_camera(), GlobalTransform::IDENTITY));

    let panel = app
        .world_mut()
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(PANEL_ORIGIN.x),
                top: Val::Px(PANEL_ORIGIN.y),
                width: Val::Px(PANEL_SIZE.x),
                height: Val::Px(PANEL_SIZE.y),
                ..default()
            },
            BackgroundColor(Color::BLACK),
            Visibility::Visible,
        ))
        .id();

    app.add_plugins(MessageProbePlugin::<MoveRequested>::default());
    for _ in 0..4 {
        app.update();
    }
    (app, panel)
}

fn panel_centre(app: &App, panel: Entity) -> Option<Vec2> {
    let transform = app.world().get::<UiGlobalTransform>(panel)?;
    let node = app.world().get::<ComputedNode>(panel)?;
    if node.size.x <= 0.0 || node.size.y <= 0.0 {
        return None;
    }
    Some(transform.translation)
}

fn place_pointer(app: &mut App, position: Vec2) {
    let Some(window) = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .iter(app.world())
        .next()
    else {
        return;
    };
    let target = RenderTarget::Window(WindowRef::Primary).normalize(Some(window));
    if let Some(target) = target {
        let mut pointers = app
            .world_mut()
            .query_filtered::<&mut PointerLocation, With<PointerId>>();
        for mut pointer in pointers.iter_mut(app.world_mut()) {
            *pointer = PointerLocation::new(Location {
                target: target.clone(),
                position,
            });
        }
    }
    let mut windows = app.world_mut().query::<&mut Window>();
    for mut window in windows.iter_mut(app.world_mut()) {
        window.set_cursor_position(Some(position));
    }
}

fn picking_sees(app: &App, entity: Entity) -> bool {
    matches!(
        app.world().get::<PickingInteraction>(entity),
        Some(PickingInteraction::Hovered | PickingInteraction::Pressed)
    )
}

fn click_once(app: &mut App) {
    clear_mouse(app);
    press_left(app);
    app.world_mut().run_schedule(Update);
    app.world_mut().run_schedule(Last);
}

fn hovered_cell(app: &App) -> bool {
    app.world()
        .get_resource::<InspectTarget>()
        .and_then(InspectTarget::hovered)
        .is_some()
}

#[test]
fn ui_picking_backend_is_already_installed() {
    let app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    assert!(
        app.is_plugin_added::<UiPickingPlugin>(),
        "bevy_ui::UiPlugin must already have added UiPickingPlugin — the `ui` feature \
         pulls `picking`, which pulls `ui_picking`",
    );
}

#[test]
fn picking_hits_a_panel_while_the_board_click_is_absorbed() {
    let level = Level::new(0);
    let (mut app, panel) = coexistence_app(level);

    let Some(centre) = panel_centre(&app, panel) else {
        unreachable!("bevy_ui must lay the probe panel out to a non-degenerate rect");
    };

    place_pointer(&mut app, centre);
    for _ in 0..2 {
        app.update();
    }

    assert!(
        picking_sees(&app, panel),
        "the UI picking backend must report an interaction on the panel — otherwise this \
         test would pass merely because picking is inert",
    );
    assert!(
        !hovered_cell(&app),
        "with the pointer over a HUD panel the battle picker must resolve NO hovered cell \
         (the GTW-380 cursor_over_ui gate), even with the picking backend live",
    );

    click_once(&mut app);
    click_once(&mut app);
    assert!(
        probed::<MoveRequested>(&app).is_empty(),
        "a two-click over the panel must emit NO MoveRequested — the UI absorbs it and the \
         picking backend adds no second path to the act bus",
    );
}

#[test]
fn picking_reports_no_hit_off_the_panel_and_the_board_click_lands() {
    let level = Level::new(0);
    let (mut app, panel) = coexistence_app(level);

    let cursor = TARGET_SIZE * 0.5 + Vec2::new(260.0, 140.0);
    assert!(
        !bevy::math::Rect::from_corners(PANEL_ORIGIN, PANEL_ORIGIN + PANEL_SIZE).contains(cursor),
        "the control cursor must lie OFF the panel",
    );

    place_pointer(&mut app, cursor);
    for _ in 0..2 {
        app.update();
    }

    assert!(
        !picking_sees(&app, panel),
        "off the panel, the picking backend must report no interaction on it",
    );
    assert!(
        hovered_cell(&app),
        "off the panel and inside the grid, the battle picker must resolve a hovered cell \
         — the live picking backend must not suppress the board pick",
    );

    click_once(&mut app);
    click_once(&mut app);
    assert_eq!(
        probed::<MoveRequested>(&app).len(),
        1,
        "the board two-click must commit EXACTLY ONE MoveRequested — a live picking \
         backend must not double-fire the act bus",
    );
    let _ = world_to_cell(Vec2::ZERO, level);
}
