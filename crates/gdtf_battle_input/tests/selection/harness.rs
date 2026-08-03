//! authoring, the intent push, and the selection / highlight probes.

use bevy::{asset::AssetPlugin, input::ButtonInput, prelude::*, scene::ScenePlugin};
use gdtf_battle_input::{
    ActIntent, GdtfBattleInputPlugin, PendingActIntent, SelectedShooter, SelectionHighlight,
};
use gdtf_battle_presenter::{ActiveLevel, ViewMode};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    ganger::LifeState,
    prelude::{BattleInProgress, CellLevel, Faction, Level, OccupancyGrid},
    vertical::VerticalLinkGraph,
};

pub(crate) const PLAYER_FACTION: Faction = Faction::new(0);
pub(crate) const ENEMY_FACTION: Faction = Faction::new(1);

pub(crate) fn mint_entity() -> Entity {
    World::new().spawn_empty().id()
}

pub(crate) fn selection_app(active_level: Level) -> App {
    use gdtf_battle_sim::tuning::CombatTuning;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin))
        .add_plugins(GdtfBattleInputPlugin);
    app.world_mut()
        .insert_resource(ActiveLevel::new(active_level));
    app.world_mut().insert_resource(ViewMode::default());
    app.world_mut().insert_resource(BattleInProgress);
    app.world_mut().insert_resource(OccupancyGrid::default());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut()
        .insert_resource(PlayerFaction::new(PLAYER_FACTION));
    app.world_mut()
        .insert_resource(VerticalLinkGraph::default());
    app.world_mut()
        .insert_resource(ButtonInput::<MouseButton>::default());
    app
}

pub(crate) fn place_player_ganger(app: &mut App, cell: CellLevel) -> Entity {
    let ganger = app.world_mut().spawn(PLAYER_FACTION).id();
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, Some(ganger));
    ganger
}

pub(crate) fn place_player_ganger_with_life(
    app: &mut App,
    cell: CellLevel,
    life: LifeState,
) -> Entity {
    let ganger = app.world_mut().spawn((PLAYER_FACTION, life)).id();
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, Some(ganger));
    ganger
}

pub(crate) fn set_hovered(app: &mut App, cell: Option<CellLevel>) {
    use gdtf_battle_input::InspectTarget;
    app.world_mut().insert_resource(InspectTarget::new(cell));
}

pub(crate) fn selected(app: &App) -> Option<Entity> {
    app.world()
        .get_resource::<SelectedShooter>()
        .and_then(|s| **s)
}

pub(crate) fn highlight_state(app: &mut App) -> Option<(Vec3, Visibility)> {
    let mut q = app
        .world_mut()
        .query_filtered::<(&Transform, &Visibility), With<SelectionHighlight>>();
    q.iter(app.world()).next().map(|(t, v)| (t.translation, *v))
}

pub(crate) fn highlight_count(app: &mut App) -> usize {
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, With<SelectionHighlight>>();
    q.iter(app.world()).count()
}

pub(crate) fn push_intent(app: &mut App, intent: ActIntent) {
    app.world_mut()
        .resource_mut::<PendingActIntent>()
        .push(intent);
}
