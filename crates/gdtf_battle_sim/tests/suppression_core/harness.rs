use bevy::{
    app::App,
    asset::AssetPlugin,
    prelude::{Entity, MinimalPlugins},
    scene::ScenePlugin,
};
use gdtf_battle_sim::{
    battle::{BattleSimPlugin, SetupBattleRequested},
    ganger::{Cool, Direction, Facing, GangRegistry, Grit, Reflexes, Speed, Suppressed, Toughness},
    metric::{Cell, CellLevel, Level},
    prelude::{Faction, Stance, StanceKind, Tu},
    rng::BattleSeed,
    situation::Situation,
    test_support::{
        GangerSpawnBuilder, test_armor_registry, test_melee_weapon_registry, test_weapon_registry,
    },
    weapon::{FireMode, Wields},
};

pub(crate) const SEED: u64 = 0x5_0BBE_7526;

pub(crate) const PLAYER: u8 = 0;
pub(crate) const ENEMY: u8 = 1;

pub(crate) fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

pub(crate) fn tuning_with_radius(radius: u8) -> gdtf_battle_sim::tuning::CombatTuning {
    use gdtf_battle_sim::tuning::{
        CombatTuning, ReactionCapBase, ReactionCapPerReactions, ReactionPMax, ReactionPMin,
        ReactionTuning, SuppressionRadius, SuppressionStabilityPenalty,
    };
    CombatTuning {
        reaction: ReactionTuning {
            cap_base:            ReactionCapBase::new(8.0),
            cap_per_reactions:   ReactionCapPerReactions::new(0.0),
            p_min:               ReactionPMin::new(1.0),
            p_max:               ReactionPMax::new(1.0),
            suppression_radius:  SuppressionRadius::new(radius),
            suppression_penalty: SuppressionStabilityPenalty::default(),
        },
        ..Default::default()
    }
}

pub(crate) fn battle_app(radius: u8) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(tuning_with_radius(radius));
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
    for _ in 0..4 {
        app.update();
    }
}

pub(crate) fn ganger(
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
        .reflexes(Reflexes::new(20.0))
        .cool(Cool::new(20.0))
        .grit(Grit::new(80.0))
        .toughness(Toughness::new(80.0))
        .build()
}

pub(crate) fn ganger_of(app: &mut App, faction: u8) -> Option<Entity> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Faction)>();
    query
        .iter(world)
        .find(|(_, f)| ***f == faction)
        .map(|(entity, _)| entity)
}

pub(crate) fn gangers_of(app: &mut App, faction: u8) -> Vec<Entity> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Faction)>();
    let mut out: Vec<Entity> = query
        .iter(world)
        .filter(|(_, f)| ***f == faction)
        .map(|(entity, _)| entity)
        .collect();
    out.sort();
    out
}

pub(crate) fn is_suppressed(app: &App, entity: Entity) -> bool {
    app.world().get::<Suppressed>(entity).is_some()
}

pub(crate) fn stance_of(app: &App, entity: Entity) -> Option<StanceKind> {
    app.world().get::<Stance>(entity).map(|s| **s)
}

pub(crate) fn tu_of(app: &App, entity: Entity) -> Option<u8> {
    app.world().get::<Tu>(entity).map(|t| **t)
}

pub(crate) fn wielded_single_mode(
    app: &mut App,
    shooter: Entity,
) -> gdtf_battle_sim::weapon::FireModeSpec {
    use bevy::ecs::relationship::RelationshipTarget as _;
    let world = app.world_mut();
    let wielded: Vec<Entity> = {
        let Some(wields) = world.get::<Wields>(shooter) else {
            unreachable!("the shooter wields weapons at setup");
        };
        wields.iter().collect()
    };
    let mode = wielded
        .into_iter()
        .find_map(|entity| world.get::<FireMode>(entity).map(FireMode::single));
    let Some(mode) = mode else {
        unreachable!("the shooter wields a ranged weapon carrying a FireMode");
    };
    mode
}

pub(crate) fn step(app: &mut App, ticks: u32) {
    for _ in 0..ticks {
        app.update();
    }
}

pub(crate) fn active_faction_is(app: &App, faction: u8) -> bool {
    app.world()
        .get_resource::<gdtf_battle_sim::turn::ActiveFaction>()
        .is_some_and(|active| ***active == faction)
}
