//! Tab/Escape respect panel focus: cycle and clear only when no panel holds focus.

use bevy::{ecs::system::RunSystemOnce, input::ButtonInput, input_focus::InputFocus, prelude::*};
use cobalt_test_utils::press_key;
use gdtf_battle_input::{
    BoundKey, GdtfBattleInputPlugin, Keybinds, PanelNavOrder, SelectedShooter, focused_panel_button,
};
use gdtf_battle_presenter::{ActiveLevel, ViewMode};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    prelude::{BattleInProgress, Cell, CellLevel, Faction, Level, OccupancyGrid},
    test_support::GangerEntityBuilder,
    vertical::VerticalLinkGraph,
};

const PLAYER_FACTION: Faction = Faction::new(0);
const LEVEL: Level = Level::new(0);

const fn test_keybinds() -> Keybinds {
    Keybinds {
        select_clear:     BoundKey::KeyEscape,
        level_up:         BoundKey::KeyPageUp,
        level_down:       BoundKey::KeyPageDown,
        toggle_full_view: BoundKey::KeyV,
        stance_cycle:     BoundKey::KeyC,
        aim_toggle:       BoundKey::KeyF,
        facing_cycle:     BoundKey::KeyR,
        select_next:      BoundKey::KeyTab,
        select_prev:      BoundKey::KeyTab,
    }
}

fn focus_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
    ))
    .add_plugins(GdtfBattleInputPlugin);
    let world = app.world_mut();
    world.insert_resource(ActiveLevel::new(LEVEL));
    world.insert_resource(ViewMode::default());
    world.insert_resource(BattleInProgress);
    world.insert_resource(OccupancyGrid::default());
    world.insert_resource(VerticalLinkGraph::default());
    world.insert_resource(PlayerFaction::new(PLAYER_FACTION));
    world.insert_resource(ButtonInput::<KeyCode>::default());
    world.insert_resource(test_keybinds());
    app
}

fn player_ganger(app: &mut App, x: i32, y: i32) -> Entity {
    GangerEntityBuilder::new()
        .faction(PLAYER_FACTION)
        .at(CellLevel::new(Cell::new(x, y), LEVEL))
        .spawn(app.world_mut())
}

fn focus_a_panel_button(app: &mut App) -> Entity {
    let button = app.world_mut().spawn(PanelNavOrder::new(0)).id();
    app.world_mut()
        .insert_resource(InputFocus::from_entity(button));
    button
}

fn selection(app: &App) -> Option<Entity> {
    **app.world().resource::<SelectedShooter>()
}

#[test]
fn tab_cycles_gangers_when_no_panel_holds_focus() {
    let mut app = focus_app();
    let g_a = player_ganger(&mut app, 0, 0);
    let g_b = player_ganger(&mut app, 1, 0);
    app.world_mut().insert_resource(SelectedShooter::new(g_a));

    press_key(&mut app, KeyCode::Tab);
    app.update();

    assert_eq!(
        selection(&app),
        Some(g_b),
        "with NO panel focused, Tab must cycle the selection to the next ganger",
    );
}

#[test]
fn tab_does_not_cycle_gangers_when_a_panel_holds_focus() {
    let mut app = focus_app();
    let g_a = player_ganger(&mut app, 0, 0);
    let _g_b = player_ganger(&mut app, 1, 0);
    app.world_mut().insert_resource(SelectedShooter::new(g_a));
    focus_a_panel_button(&mut app);

    press_key(&mut app, KeyCode::Tab);
    app.update();

    assert_eq!(
        selection(&app),
        Some(g_a),
        "with a panel focused, Tab must NOT cycle gangers (the guard skips \
         cycle_selection_keys — Tab drives panel focus-nav instead)",
    );
}

#[test]
fn escape_clears_selection_when_no_panel_holds_focus() {
    let mut app = focus_app();
    let g_a = player_ganger(&mut app, 0, 0);
    app.world_mut().insert_resource(SelectedShooter::new(g_a));

    press_key(&mut app, KeyCode::Escape);
    app.update();

    assert_eq!(
        selection(&app),
        None,
        "with NO panel focused, Escape must clear the selection as before",
    );
}

#[test]
fn escape_does_not_clear_selection_when_a_panel_holds_focus() {
    let mut app = focus_app();
    let g_a = player_ganger(&mut app, 0, 0);
    app.world_mut().insert_resource(SelectedShooter::new(g_a));
    focus_a_panel_button(&mut app);

    press_key(&mut app, KeyCode::Escape);
    app.update();

    assert_eq!(
        selection(&app),
        Some(g_a),
        "with a panel focused, Escape must NOT clear the selection (the guard skips \
         select_clear_key — Escape cancels panel focus instead)",
    );
}

fn probe_focus(
    focus: Option<Res<InputFocus>>,
    panels: Query<(), With<PanelNavOrder>>,
) -> Option<Entity> {
    focused_panel_button(focus.as_deref(), &panels)
}

fn panel_focus(app: &mut App) -> Option<Entity> {
    app.world_mut().run_system_once(probe_focus).unwrap_or(None)
}

#[test]
fn focused_panel_button_reports_only_panel_focus() {
    let mut app = focus_app();

    assert_eq!(
        panel_focus(&mut app),
        None,
        "no InputFocus → no panel focus"
    );

    let stray = app.world_mut().spawn_empty().id();
    app.world_mut()
        .insert_resource(InputFocus::from_entity(stray));
    assert_eq!(
        panel_focus(&mut app),
        None,
        "focus on a non-PanelNavOrder entity → no panel focus",
    );

    let button = focus_a_panel_button(&mut app);
    assert_eq!(
        panel_focus(&mut app),
        Some(button),
        "focus on a PanelNavOrder button → that button is the panel-focus target",
    );
}
