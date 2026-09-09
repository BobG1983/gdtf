use bevy::{
    app::App,
    prelude::{Entity, MinimalPlugins},
    scene::ScenePlugin,
};
use cobalt_test_utils::unwatched_asset_plugin;
use gdtf_battle_sim::{
    acts::movement::WalkInProgress,
    battle::{BattleSimPlugin, SetupBattleRequested},
    floor::FloorCostGrid,
    ganger::{GangRegistry, Speed},
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    pathfinder::{MoveGrids, PlanningView, find_path},
    prelude::{Faction, Position, Stance, StanceKind, Tu},
    rng::BattleSeed,
    situation::Situation,
    test_support::{
        GangerSpawnBuilder, SituationBuilder, test_armor_registry, test_melee_weapon_registry,
        test_weapon_registry,
    },
    tuning::{CombatTuning, ReactionCapBase, ReactionCapPerReactions, ReactionTuning, ViewRange},
    vertical::VerticalLinkGraph,
    visibility::{FactionRelation, SquadVisibility},
};

pub(crate) const SEED: u64 = 0x5A1C_AC75;

pub(crate) const PLAYER: u8 = 0;
pub(crate) const ENEMY: u8 = 1;

pub(crate) const TEST_VIEW_RANGE: u16 = 4;

pub(crate) fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

pub(crate) fn player_at() -> CellLevel {
    ground(5, 5)
}

pub(crate) fn battle_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, unwatched_asset_plugin(), ScenePlugin));
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(CombatTuning {
        view_range: ViewRange::new(TEST_VIEW_RANGE),
        reaction: ReactionTuning {
            cap_base: ReactionCapBase::new(0.0),
            cap_per_reactions: ReactionCapPerReactions::new(0.0),
            ..Default::default()
        },
        ..Default::default()
    });
    app.insert_resource(test_weapon_registry());
    app.insert_resource(test_melee_weapon_registry());
    app.insert_resource(test_armor_registry());
    app
}

pub(crate) fn drive_setup(app: &mut App, situation_and_gangs: (Situation, GangRegistry)) {
    let (situation, gangs) = situation_and_gangs;
    app.world_mut().insert_resource(gangs);
    app.world_mut()
        .write_message(SetupBattleRequested::new(situation, BattleSeed::new(SEED)));
    app.update();
    app.update();
    app.update();
}

pub(crate) fn player_entity(app: &mut App) -> Option<Entity> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Faction)>();
    query
        .iter(world)
        .find(|(_, faction)| ***faction == PLAYER)
        .map(|(entity, _)| entity)
}

pub(crate) fn pos_and_tu(app: &App, entity: Entity) -> Option<(CellLevel, u8)> {
    let position = app.world().get::<Position>(entity).map(|p| **p)?;
    let tu = app.world().get::<Tu>(entity).map(|t| **t)?;
    Some((position, tu))
}

pub(crate) fn is_walking(app: &App, entity: Entity) -> bool {
    app.world().get::<WalkInProgress>(entity).is_some()
}

pub(crate) fn plan_total(app: &App, start: CellLevel, goal: CellLevel) -> Option<u8> {
    let grid = app.world().get_resource::<OccupancyGrid>()?;
    let links = app.world().get_resource::<VerticalLinkGraph>()?;
    let squad = app.world().get_resource::<SquadVisibility>()?;
    let tuning = app.world().get_resource::<CombatTuning>()?;
    let floor_costs = app.world().get_resource::<FloorCostGrid>()?;
    let planning = PlanningView::new(squad, |_occupant| FactionRelation::Other);
    let path = find_path(
        start,
        goal,
        MoveGrids {
            occupancy: grid,
            links,
            floor_costs,
            tuning,
        },
        gdtf_battle_sim::injuries::MovementCostFactor::IDENTITY,
        &planning,
    )
    .ok()?;
    Some(*path.total())
}

pub(crate) fn run_until_walk_ends(app: &mut App, entity: Entity) {
    for _ in 0..32 {
        app.update();
        if !is_walking(app, entity) {
            return;
        }
    }
}

pub(crate) fn one_player_situation(speed: f32) -> (Situation, GangRegistry) {
    SituationBuilder::new()
        .with_gangers([GangerSpawnBuilder::new()
            .at(player_at())
            .faction(Faction::new(PLAYER))
            .stance(Stance::new(StanceKind::Standing))
            .speed(Speed::new(speed))
            .build()])
        .build_with_gangs()
}

pub(crate) fn player_and_enemy_situation(
    speed: f32,
    enemy_cell: CellLevel,
) -> (Situation, GangRegistry) {
    SituationBuilder::new()
        .with_gangers([
            GangerSpawnBuilder::new()
                .at(player_at())
                .faction(Faction::new(PLAYER))
                .stance(Stance::new(StanceKind::Standing))
                .speed(Speed::new(speed))
                .build(),
            GangerSpawnBuilder::new()
                .at(enemy_cell)
                .faction(Faction::new(ENEMY))
                .stance(Stance::new(StanceKind::Standing))
                .speed(Speed::new(speed))
                .build(),
        ])
        .build_with_gangs()
}
