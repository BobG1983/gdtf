use bevy::{input::ButtonInput, prelude::*};
use gdtf_battle_input::{ActIntent, GdtfBattleInputPlugin, PendingActIntent, SelectedShooter};
use gdtf_battle_presenter::{ActiveLevel, ViewMode};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    ganger::LifeState,
    prelude::{BattleInProgress, Cell, CellLevel, Faction, Level, OccupancyGrid},
    test_support::GangerEntityBuilder,
    vertical::VerticalLinkGraph,
};

const PLAYER_FACTION: Faction = Faction::new(0);
const ENEMY_FACTION: Faction = Faction::new(1);
const LEVEL: Level = Level::new(0);

#[derive(Resource, Default)]
struct SelectionChangeCount(u32);

fn record_selection_changes(
    selected: Res<SelectedShooter>,
    mut count: ResMut<SelectionChangeCount>,
) {
    if selected.is_changed() {
        count.0 += 1;
    }
}

fn select_app() -> App {
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
    app.init_resource::<SelectionChangeCount>();
    app.add_systems(Last, record_selection_changes);
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
fn select_intent_selects_player_faction_ganger() {
    let mut app = select_app();
    let g_first = placed_ganger(&mut app, PLAYER_FACTION, 0, 0);
    let g_target = placed_ganger(&mut app, PLAYER_FACTION, 5, 5);
    app.world_mut()
        .insert_resource(SelectedShooter::new(g_first));

    push(&mut app, ActIntent::Select(g_target));
    app.update();

    assert_eq!(
        selection(&app),
        Some(g_target),
        "Select(player ganger) sets SelectedShooter to the carried entity",
    );
}

#[test]
fn select_intent_refuses_enemy_faction_ganger() {
    let mut app = select_app();
    let g_player = placed_ganger(&mut app, PLAYER_FACTION, 0, 0);
    let g_enemy = placed_ganger(&mut app, ENEMY_FACTION, 5, 5);
    app.world_mut()
        .insert_resource(SelectedShooter::new(g_player));

    push(&mut app, ActIntent::Select(g_enemy));
    app.update();

    assert_eq!(
        selection(&app),
        Some(g_player),
        "Select(enemy ganger) is REFUSED — the player selection is left untouched",
    );
}

#[test]
fn select_intent_refuses_downed_player_ganger() {
    let mut app = select_app();
    let g_alive = placed_ganger(&mut app, PLAYER_FACTION, 0, 0);
    let g_downed = placed_downed_ganger(&mut app, PLAYER_FACTION, 5, 5);
    app.world_mut()
        .insert_resource(SelectedShooter::new(g_alive));

    push(&mut app, ActIntent::Select(g_downed));
    app.update();

    assert_eq!(
        selection(&app),
        Some(g_alive),
        "Select(Downed player ganger) is REFUSED — a Downed ganger is never a selectable actor",
    );
}

#[test]
fn reselecting_current_shooter_is_change_detection_noop() {
    let mut app = select_app();
    let g_player = placed_ganger(&mut app, PLAYER_FACTION, 0, 0);
    app.world_mut()
        .insert_resource(SelectedShooter::new(g_player));

    app.update();
    app.world_mut().resource_mut::<SelectionChangeCount>().0 = 0;

    push(&mut app, ActIntent::Select(g_player));
    app.update();

    assert_eq!(
        selection(&app),
        Some(g_player),
        "re-selecting the current shooter keeps the same selection",
    );
    assert_eq!(
        app.world().resource::<SelectionChangeCount>().0,
        0,
        "re-selecting the current shooter must not spuriously trip Changed<SelectedShooter>",
    );
}

#[test]
fn select_intent_refuses_dead_entity_token() {
    let mut app = select_app();
    let g_keep = placed_ganger(&mut app, PLAYER_FACTION, 0, 0);
    let g_victim = placed_ganger(&mut app, PLAYER_FACTION, 5, 5);
    app.world_mut()
        .insert_resource(SelectedShooter::new(g_keep));
    app.world_mut().despawn(g_victim);

    push(&mut app, ActIntent::Select(g_victim));
    app.update();

    assert_eq!(
        selection(&app),
        Some(g_keep),
        "Select(dead token) is REFUSED (fail-closed, no panic) — the selection is untouched",
    );
}
