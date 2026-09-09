use bevy::{
    app::App,
    prelude::{Entity, MinimalPlugins, Resource},
    scene::ScenePlugin,
};
use cobalt_test_utils::{advance_until, unwatched_asset_plugin};
use gdtf_battle_sim::{
    acts::{EndTurnRequested, movement::WalkInProgress},
    battle::{BattleSimPlugin, SetupBattleRequested},
    ganger::{Cool, Direction, Facing, GangRegistry, Reflexes, Speed},
    metric::{Cell, CellLevel, Level},
    prelude::{Faction, Position, Stance, StanceKind, Tu},
    rng::BattleSeed,
    shot_fired::ShotFired,
    situation::{PlacedGanger, Situation},
    test_support::{
        GangerSpawnBuilder, test_armor_registry, test_melee_weapon_registry, test_weapon_registry,
    },
    tuning::{
        CombatTuning, ReactionCapBase, ReactionCapPerReactions, ReactionPMax, ReactionPMin,
        ReactionTuning, ReactionsUsed, SuppressionRadius, SuppressionStabilityPenalty, ViewRange,
    },
};

pub(crate) const SEED: u64 = 0x4EAC_7104;

pub(crate) const PLAYER: u8 = 0;
pub(crate) const ENEMY: u8 = 1;

pub(crate) const TEST_VIEW_RANGE: u16 = 6;

pub(crate) fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

pub(crate) const fn forced_reaction_tuning(cap: u32) -> ReactionTuning {
    ReactionTuning {
        cap_base:            ReactionCapBase::new(cap as f32),
        cap_per_reactions:   ReactionCapPerReactions::new(0.0),
        p_min:               ReactionPMin::new(1.0),
        p_max:               ReactionPMax::new(1.0),
        suppression_radius:  SuppressionRadius::new(0),
        suppression_penalty: SuppressionStabilityPenalty::new(0.0),
    }
}

pub(crate) fn battle_app(reaction: ReactionTuning) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, unwatched_asset_plugin(), ScenePlugin));
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(CombatTuning {
        view_range: ViewRange::new(TEST_VIEW_RANGE),
        reaction,
        ..Default::default()
    });
    app.insert_resource(test_weapon_registry());
    app.insert_resource(test_melee_weapon_registry());
    app.insert_resource(test_armor_registry());
    app
}

pub(crate) fn drive_setup(
    app: &mut App,
    situation_and_gangs: (Situation, Vec<PlacedGanger>, GangRegistry),
) {
    let (situation, placements, gangs) = situation_and_gangs;
    app.world_mut().insert_resource(gangs);
    app.world_mut().write_message(SetupBattleRequested::new(
        situation,
        placements,
        BattleSeed::new(SEED),
    ));
    for _ in 0..4 {
        app.update();
    }
}

pub(crate) fn ganger_of(app: &mut App, faction: u8) -> Option<Entity> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Faction)>();
    query
        .iter(world)
        .find(|(_, f)| ***f == faction)
        .map(|(entity, _)| entity)
}

pub(crate) fn tu_of(app: &App, entity: Entity) -> Option<u8> {
    app.world().get::<Tu>(entity).map(|t| **t)
}

pub(crate) fn pos_of(app: &App, entity: Entity) -> Option<CellLevel> {
    app.world().get::<Position>(entity).map(|p| **p)
}

pub(crate) fn used_of(app: &App, entity: Entity) -> Option<u32> {
    app.world().get::<ReactionsUsed>(entity).map(|u| **u)
}

pub(crate) fn is_walking(app: &App, entity: Entity) -> bool {
    app.world().get::<WalkInProgress>(entity).is_some()
}

#[derive(Resource, Default)]
pub(crate) struct ShotLog {
    rounds: Vec<(Entity, bool)>,
}

pub(crate) fn record_shots(
    mut shots: bevy::prelude::MessageReader<ShotFired>,
    mut log: bevy::prelude::ResMut<ShotLog>,
) {
    for shot in shots.read() {
        log.rounds.push((shot.shooter, shot.report.is_some()));
    }
}

pub(crate) fn with_shot_log(app: &mut App) {
    app.init_resource::<ShotLog>();
    app.add_systems(bevy::app::Update, record_shots);
}

pub(crate) fn shots_by(app: &App, shooter: Entity) -> usize {
    app.world().get_resource::<ShotLog>().map_or(0, |log| {
        log.rounds.iter().filter(|(s, _)| *s == shooter).count()
    })
}

pub(crate) fn all_shots_have_reports(app: &App, shooter: Entity) -> bool {
    app.world().get_resource::<ShotLog>().is_some_and(|log| {
        let mut any = false;
        let ok = log
            .rounds
            .iter()
            .filter(|(s, _)| *s == shooter)
            .all(|(_, has)| {
                any = true;
                *has
            });
        any && ok
    })
}

pub(crate) fn watcher(
    at: CellLevel,
    faction: u8,
    facing: Direction,
) -> gdtf_battle_sim::situation::GangerSpawn {
    use gdtf_battle_sim::ganger::{Grit, Toughness};
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(faction))
        .facing(Facing::new(facing))
        .stance(Stance::new(StanceKind::Standing))
        .speed(Speed::new(20.0))
        .reflexes(Reflexes::new(20.0))
        .cool(Cool::new(20.0))
        .grit(Grit::new(80.0))
        .toughness(Toughness::new(80.0))
        .build()
}

pub(crate) fn mover(
    at: CellLevel,
    faction: u8,
    facing: Direction,
) -> gdtf_battle_sim::situation::GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(faction))
        .facing(Facing::new(facing))
        .stance(Stance::new(StanceKind::Standing))
        .speed(Speed::new(20.0))
        .build()
}

pub(crate) fn tough_mover(
    at: CellLevel,
    faction: u8,
    facing: Direction,
) -> gdtf_battle_sim::situation::GangerSpawn {
    use gdtf_battle_sim::ganger::{Grit, Toughness};
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(faction))
        .facing(Facing::new(facing))
        .stance(Stance::new(StanceKind::Standing))
        .speed(Speed::new(20.0))
        .grit(Grit::new(80.0))
        .toughness(Toughness::new(80.0))
        .build()
}

pub(crate) fn run_until_walk_ends(app: &mut App, entity: Entity) {
    advance_until(app, |app| !is_walking(app, entity));
}

pub(crate) fn step(app: &mut App, ticks: u32) {
    for _ in 0..ticks {
        app.update();
    }
}

pub(crate) fn player_turn_active(app: &App) -> bool {
    app.world()
        .get_resource::<gdtf_battle_sim::turn::ActiveFaction>()
        .is_some_and(|active| ***active == PLAYER)
}

pub(crate) fn cycle_back_to_player_turn(app: &mut App) {
    app.world_mut().write_message(EndTurnRequested);
    advance_until(app, player_turn_active);
}
