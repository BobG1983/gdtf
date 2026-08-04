//! Selection cycle: next/prev by cell order, skip enemies and downed.
use bevy::{input::ButtonInput, prelude::*};
use gdtf_battle_input::{
    ActIntent, BoundKey, GdtfBattleInputPlugin, Keybinds, PendingActIntent, SelectedShooter,
};
use gdtf_battle_presenter::{ActiveLevel, ViewMode};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    ganger::LifeState,
    prelude::{BattleInProgress, Cell, CellLevel, Faction, Level, OccupancyGrid},
    test_support::GangerEntityBuilder,
    vertical::VerticalLinkGraph,
};
use gdtf_test_utils::{clear_keys, press_key};

const PLAYER_FACTION: Faction = Faction::new(0);
const ENEMY_FACTION: Faction = Faction::new(1);
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

fn cycle_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
    ))
    .add_plugins(GdtfBattleInputPlugin);
    app.world_mut().insert_resource(ActiveLevel::new(LEVEL));
    app.world_mut().insert_resource(ViewMode::default());
    app.world_mut().insert_resource(BattleInProgress);
    app.world_mut().insert_resource(OccupancyGrid::default());
    app.world_mut()
        .insert_resource(VerticalLinkGraph::default());
    app.world_mut()
        .insert_resource(PlayerFaction::new(PLAYER_FACTION));
    app.world_mut()
        .insert_resource(ButtonInput::<KeyCode>::default());
    app
}

fn placed_ganger(app: &mut App, faction: Faction, x: i32, y: i32) -> Entity {
    GangerEntityBuilder::new()
        .faction(faction)
        .at(CellLevel::new(Cell::new(x, y), LEVEL))
        .spawn(app.world_mut())
}

fn placed_downed_ganger(app: &mut App, faction: Faction, x: i32, y: i32) -> Entity {
    GangerEntityBuilder::new()
        .faction(faction)
        .life_state(LifeState::Downed)
        .at(CellLevel::new(Cell::new(x, y), LEVEL))
        .spawn(app.world_mut())
}

fn push(app: &mut App, intent: ActIntent) {
    app.world_mut()
        .resource_mut::<PendingActIntent>()
        .push(intent);
}

fn selection(app: &App) -> Option<Entity> {
    **app.world().resource::<SelectedShooter>()
}

#[test]
fn select_next_advances_and_wraps_in_cell_order() {
    let mut app = cycle_app();
    let g_a = placed_ganger(&mut app, PLAYER_FACTION, 0, 0);
    let g_b = placed_ganger(&mut app, PLAYER_FACTION, 1, 0);
    let g_c = placed_ganger(&mut app, PLAYER_FACTION, 0, 1);
    app.world_mut().insert_resource(SelectedShooter::new(g_a));

    push(&mut app, ActIntent::SelectNext);
    app.update();
    assert_eq!(selection(&app), Some(g_b), "Next from g_a -> g_b");

    push(&mut app, ActIntent::SelectNext);
    app.update();
    assert_eq!(selection(&app), Some(g_c), "Next from g_b -> g_c");

    push(&mut app, ActIntent::SelectNext);
    app.update();
    assert_eq!(selection(&app), Some(g_a), "Next from g_c WRAPS to g_a");
}

#[test]
fn select_prev_advances_and_wraps_in_cell_order() {
    let mut app = cycle_app();
    let g_a = placed_ganger(&mut app, PLAYER_FACTION, 0, 0);
    let g_b = placed_ganger(&mut app, PLAYER_FACTION, 1, 0);
    let g_c = placed_ganger(&mut app, PLAYER_FACTION, 0, 1);
    app.world_mut().insert_resource(SelectedShooter::new(g_a));

    push(&mut app, ActIntent::SelectPrev);
    app.update();
    assert_eq!(selection(&app), Some(g_c), "Prev from g_a WRAPS to g_c");

    push(&mut app, ActIntent::SelectPrev);
    app.update();
    assert_eq!(selection(&app), Some(g_b), "Prev from g_c -> g_b");

    push(&mut app, ActIntent::SelectPrev);
    app.update();
    assert_eq!(selection(&app), Some(g_a), "Prev from g_b -> g_a");
}

#[test]
fn cycle_steps_cell_order_not_spawn_or_entity_id_order() {
    let mut app = cycle_app();
    let mid = placed_ganger(&mut app, PLAYER_FACTION, 1, 1);
    let lo = placed_ganger(&mut app, PLAYER_FACTION, 0, 0);
    let hi = placed_ganger(&mut app, PLAYER_FACTION, 9, 9);
    app.world_mut().insert_resource(SelectedShooter::new(lo));

    push(&mut app, ActIntent::SelectNext);
    app.update();
    assert_eq!(
        selection(&app),
        Some(mid),
        "Next from the cell-order first (lo) steps the (z, y, x) cell order to mid — NOT a \
         spawn/iteration-order pick (which would skip to hi)",
    );
    assert_ne!(
        selection(&app),
        Some(hi),
        "a spawn/iteration-order or Entity-id step (which would land on hi) must NOT win — only \
         the deterministic cell order",
    );

    push(&mut app, ActIntent::SelectNext);
    app.update();
    assert_eq!(
        selection(&app),
        Some(hi),
        "Next from mid steps to hi (cell-order last)"
    );

    push(&mut app, ActIntent::SelectNext);
    app.update();
    assert_eq!(
        selection(&app),
        Some(lo),
        "Next from hi WRAPS to lo (cell-order first) — the walk is the cell order",
    );
}

#[test]
fn cycle_ignores_enemy_gangers() {
    let mut app = cycle_app();
    let p_a = placed_ganger(&mut app, PLAYER_FACTION, 0, 0);
    let _enemy_low = placed_ganger(&mut app, ENEMY_FACTION, 1, 0);
    let p_b = placed_ganger(&mut app, PLAYER_FACTION, 2, 0);
    let _enemy_high = placed_ganger(&mut app, ENEMY_FACTION, 3, 0);
    app.world_mut().insert_resource(SelectedShooter::new(p_a));

    push(&mut app, ActIntent::SelectNext);
    app.update();
    assert_eq!(selection(&app), Some(p_b), "Next skips the enemy to p_b");

    push(&mut app, ActIntent::SelectNext);
    app.update();
    assert_eq!(
        selection(&app),
        Some(p_a),
        "Next WRAPS p_b -> p_a (skips enemies)"
    );
}

#[test]
fn cycle_skips_downed_gangers() {
    let mut app = cycle_app();
    let p_a = placed_ganger(&mut app, PLAYER_FACTION, 0, 0);
    let downed = placed_downed_ganger(&mut app, PLAYER_FACTION, 1, 0);
    let p_b = placed_ganger(&mut app, PLAYER_FACTION, 2, 0);
    app.world_mut().insert_resource(SelectedShooter::new(p_a));

    push(&mut app, ActIntent::SelectNext);
    app.update();
    assert_eq!(
        selection(&app),
        Some(p_b),
        "Next SKIPS the Downed ganger (cell-order between p_a and p_b) straight to p_b",
    );
    assert_ne!(
        selection(&app),
        Some(downed),
        "the cycle must NEVER land on a Downed ganger",
    );

    push(&mut app, ActIntent::SelectNext);
    app.update();
    assert_eq!(
        selection(&app),
        Some(p_a),
        "Next WRAPS p_b -> p_a, still skipping the Downed ganger",
    );
}

#[test]
fn empty_player_gang_cycle_is_noop() {
    let mut app = cycle_app();
    let _enemy = placed_ganger(&mut app, ENEMY_FACTION, 0, 0);
    app.world_mut().insert_resource(SelectedShooter::cleared());

    push(&mut app, ActIntent::SelectNext);
    app.update();
    assert_eq!(
        selection(&app),
        None,
        "Next over an empty player gang is a no-op"
    );

    push(&mut app, ActIntent::SelectPrev);
    app.update();
    assert_eq!(
        selection(&app),
        None,
        "Prev over an empty player gang is a no-op"
    );
}

#[test]
fn tab_cycles_next_and_shift_tab_cycles_prev() {
    let mut app = cycle_app();
    app.world_mut().insert_resource(test_keybinds());
    let g_a = placed_ganger(&mut app, PLAYER_FACTION, 0, 0);
    let g_b = placed_ganger(&mut app, PLAYER_FACTION, 1, 0);
    placed_ganger(&mut app, PLAYER_FACTION, 0, 1);
    app.world_mut().insert_resource(SelectedShooter::new(g_a));

    press_key(&mut app, KeyCode::Tab);
    app.update();
    assert_eq!(selection(&app), Some(g_b), "Tab cycles to the NEXT ganger");
    clear_keys(&mut app);

    press_key(&mut app, KeyCode::ShiftLeft);
    press_key(&mut app, KeyCode::Tab);
    app.update();
    assert_eq!(
        selection(&app),
        Some(g_a),
        "Shift+Tab cycles to the PREVIOUS ganger"
    );
    clear_keys(&mut app);
}
