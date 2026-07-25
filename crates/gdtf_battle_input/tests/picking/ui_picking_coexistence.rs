//! GTW-811 SPIKE PROBE — Bevy's UI picking backend and the battle `cursor_over_ui`
//! arbitration run side by side without fighting.
//!
//! The spike question was whether ENABLING `UiPickingPlugin` would regress
//! `gdtf_battle_input`'s over-UI gate. The plugin is ALREADY installed in every build here —
//! `bevy`'s `ui` feature pulls `picking`, which pulls `ui_picking`, and
//! `bevy_ui::UiPlugin::build` adds `UiPickingPlugin` under that feature — so the real
//! question is whether the two pipelines already coexist. The three tests
//! ([`ui_picking_backend_is_already_installed`],
//! [`picking_hits_a_panel_while_the_board_click_is_absorbed`] and its control
//! [`picking_reports_no_hit_off_the_panel_and_the_board_click_lands`]) answer it by
//! observation: the backend is present and demonstrably LIVE, a click over a HUD panel still
//! reaches no act, and the same pointer off the panel commits exactly one.
//!
//! Every world mutation is in a TEST BODY — the accepted headless idiom
//! (`bevy-traps.md` #7 carve-out (a)). See
//! `docs/ui-picking-arbitration.md` for the full finding.

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

/// The faction the player controls in this probe (matches the seeded `PlayerFaction`).
const PLAYER_FACTION: Faction = Faction::new(0);

/// The synthetic HUD panel's size in logical px — a bottom-bar-sized slab.
const PANEL_SIZE: Vec2 = Vec2::new(240.0, 140.0);

/// The panel's top-left offset from the window origin, in logical px. Chosen well inside
/// the window so the panel's centre also unprojects to an in-grid board cell.
const PANEL_ORIGIN: Vec2 = Vec2::new(520.0, 290.0);

/// Builds the coexistence probe app: the real `DefaultPlugins` UI/asset harness (which
/// brings `UiPlugin`, and therefore `UiPickingPlugin` and the `DefaultPickingPlugins`
/// pointer pipeline) plus the real `GdtfBattleInputPlugin` and the battle resources its
/// click systems gate on, a `PrimaryWindow`, a synthesized `WorldCamera`, and a real
/// `Node` panel that `bevy_ui` lays out for itself.
///
/// Returns the app together with the panel entity.
fn coexistence_app(level: Level) -> (App, Entity) {
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    app.add_plugins(GdtfBattleInputPlugin);

    // The presenter normally owns these; the probe inserts them directly.
    app.world_mut().insert_resource(ActiveLevel::new(level));
    app.world_mut().insert_resource(ViewMode::default());
    app.world_mut().insert_resource(BattleInProgress);
    app.world_mut().insert_resource(OccupancyGrid::default());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut()
        .insert_resource(PlayerFaction::new(PLAYER_FACTION));
    app.world_mut()
        .insert_resource(VerticalLinkGraph::default());

    // A player-faction ganger with no firing components: FIRE fails closed, so a click on
    // an empty in-bounds cell resolves to the MOVE rung.
    let ganger = app.world_mut().spawn(PLAYER_FACTION).id();
    app.world_mut()
        .insert_resource(gdtf_battle_input::SelectedShooter::new(ganger));

    // The single primary window the picker reads the cursor from.
    app.world_mut().spawn((
        Window {
            resolution: WindowResolution::new(TARGET_SIZE.x as u32, TARGET_SIZE.y as u32),
            ..default()
        },
        PrimaryWindow,
    ));

    // The world camera: a deterministic projection at the identity transform, separate
    // from the harness's own UI camera so the board math stays independent of UI layout.
    app.world_mut()
        .spawn((WorldCamera, synthetic_camera(), GlobalTransform::IDENTITY));

    // A REAL `bevy_ui` panel: laid out by `ui_layout_system`, stacked by `ui_stack_system`,
    // and therefore visible to the UI picking backend (a hand-synthesized `ComputedNode`
    // would be invisible to it, since picking walks the `UiStack`).
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
    // Settle layout + the picking pipeline before any assertion.
    for _ in 0..4 {
        app.update();
    }
    (app, panel)
}

/// The panel's laid-out centre in logical px, read back from `bevy_ui` (never predicted).
fn panel_centre(app: &App, panel: Entity) -> Option<Vec2> {
    let transform = app.world().get::<UiGlobalTransform>(panel)?;
    let node = app.world().get::<ComputedNode>(panel)?;
    // `UiGlobalTransform` is PHYSICAL px; the probe's window scale factor is 1.0, so
    // physical == logical here. Guard against a degenerate (unlaid-out) node.
    if node.size.x <= 0.0 || node.size.y <= 0.0 {
        return None;
    }
    Some(transform.translation)
}

/// Points the mouse pointer entity `bevy_picking` spawned at `position` on the primary
/// window, and sets the OS cursor to the same place, so BOTH pipelines see one pointer.
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

/// Whether the UI picking pipeline currently reports an interaction on `entity` — the
/// witness that the backend is LIVE rather than inert.
fn picking_sees(app: &App, entity: Entity) -> bool {
    matches!(
        app.world().get::<PickingInteraction>(entity),
        Some(PickingInteraction::Hovered | PickingInteraction::Pressed)
    )
}

/// Synthesizes ONE left-click edge and ticks the frame.
///
/// `DefaultPlugins` brings `InputPlugin`, whose `PreUpdate` clear would wipe a
/// synthesized just-pressed edge before `Update` ever saw it, so this runs `Update` (the
/// schedule the click systems live in) and `Last` (where the message probe drains) by hand
/// rather than calling `App::update` — the same reason the UI-click suites do.
fn click_once(app: &mut App) {
    clear_mouse(app);
    press_left(app);
    app.world_mut().run_schedule(Update);
    app.world_mut().run_schedule(Last);
}

/// The battle picker's live hovered cell.
fn hovered_cell(app: &App) -> bool {
    app.world()
        .get_resource::<InspectTarget>()
        .and_then(InspectTarget::hovered)
        .is_some()
}

/// GTW-811 finding 1 — the UI picking backend is ALREADY installed by `bevy_ui::UiPlugin`
/// in the same `DefaultPlugins` set the game runs, with no manifest change.
///
/// This is the fact that dissolves the ticket's premise: there is no "enable
/// `UiPickingPlugin`" step to take or to fear, because `bevy`'s `ui` feature pulls
/// `picking` → `ui_picking`, and `UiPlugin::build` adds the backend under it.
#[test]
fn ui_picking_backend_is_already_installed() {
    let app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    assert!(
        app.is_plugin_added::<UiPickingPlugin>(),
        "bevy_ui::UiPlugin must already have added UiPickingPlugin — the `ui` feature \
         pulls `picking`, which pulls `ui_picking`",
    );
}

/// GTW-811 finding 2 (the regression evidence, C10) — with the UI picking backend LIVE and
/// reporting an interaction on the panel, the battle `cursor_over_ui` gate still absorbs
/// the click: no hovered cell, and NO `MoveRequested` from a two-click over the panel.
///
/// Pin-discriminating against the sibling control test below, which uses the same app and
/// the same pointer to prove a click OFF the panel does commit exactly one move.
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

/// GTW-811 finding 2 control — the SAME live picking pipeline, with the pointer moved OFF
/// the panel onto bare board, reports no panel interaction and the board two-click commits
/// EXACTLY ONE `MoveRequested`.
///
/// This is the no-double-fire half: a live picking backend must neither suppress the board
/// act nor duplicate it.
#[test]
fn picking_reports_no_hit_off_the_panel_and_the_board_click_lands() {
    let level = Level::new(0);
    let (mut app, panel) = coexistence_app(level);

    // A cursor clear of the panel whose world point floors into the grid. The camera sits
    // at the identity, so an in-grid cell (world x > 0, world y < 0) needs a cursor
    // down-and-right of the window centre; this one is past the panel's right edge.
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
    // Name the projection helper so the in-grid premise is compile-proven reachable.
    let _ = world_to_cell(Vec2::ZERO, level);
}
