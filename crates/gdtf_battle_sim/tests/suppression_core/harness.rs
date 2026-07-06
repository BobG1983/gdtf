//! Shared GTW-526 suppression fixture: the radius tuning, the live battle-app
//! driver, the ganger builders / accessors, and turn stepping.

use bevy::{
    app::App,
    asset::AssetPlugin,
    prelude::{Entity, MinimalPlugins},
    scene::ScenePlugin,
};
use gdtf_battle_sim::{
    Faction, Stance, StanceKind, Suppressed, Tu,
    battle::{BattleSimPlugin, SetupBattleRequested},
    ganger::{Cool, Direction, Facing, GangRegistry, Grit, Reflexes, Speed, Toughness},
    metric::{Cell, CellLevel, Level},
    rng::BattleSeed,
    situation::Situation,
    test_support::{
        GangerSpawnBuilder, test_armor_registry, test_melee_weapon_registry, test_weapon_registry,
    },
    weapon::{FireMode, Wields},
};

/// An arbitrary (not shipped tuning) seed for the test battle's RNG streams.
pub(crate) const SEED: u64 = 0x5_0BBE_7526;

/// Gang `0` is the player; gang `1` is the enemy.
pub(crate) const PLAYER: u8 = 0;
pub(crate) const ENEMY: u8 = 1;

/// A ground-floor `(cell, level)` key.
pub(crate) fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// A `CombatTuning` with the given `suppression_radius` (leaving the rest default), plus a
/// forced reaction clamp `p_min == p_max == 1.0` so the (b) control reactor is GUARANTEED
/// to interrupt — the determinism proof needs a live draw in the control.
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
            // The core-slice tests are agnostic to the shot-cone penalty (they assert the
            // component / RNG-draw / clear cadence, not stability); keep the default.
            suppression_penalty: SuppressionStabilityPenalty::default(),
        },
        ..Default::default()
    }
}

/// Build the FULL live-runtime harness (the `reaction_trigger` `battle_app` idiom) with the given
/// suppression radius, plus the persistent `Load` weapon/armor registries a
/// `MinimalPlugins` app has no `AssetServer` to load.
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

/// Drive a setup through the REAL `setup_battle_on_request` Ok path and settle it.
pub(crate) fn drive_setup(app: &mut App, situation_and_gangs: (Situation, GangRegistry)) {
    let (situation, gangs) = situation_and_gangs;
    app.world_mut().insert_resource(gangs);
    app.world_mut()
        .write_message(SetupBattleRequested::new(situation, BattleSeed::new(SEED)));
    for _ in 0..4 {
        app.update();
    }
}

/// A standing ganger at `at` of `faction`, facing `facing`, with an ample TU pool + a fat
/// HP/Wounds pool (so an incidental shot cannot down it before the assertions complete).
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

/// The entity of the (first) ganger of `faction`.
pub(crate) fn ganger_of(app: &mut App, faction: u8) -> Option<Entity> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Faction)>();
    query
        .iter(world)
        .find(|(_, f)| ***f == faction)
        .map(|(entity, _)| entity)
}

/// Every ganger of `faction`, in spawn order.
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

/// Whether `entity` currently carries the `Suppressed` marker.
pub(crate) fn is_suppressed(app: &App, entity: Entity) -> bool {
    app.world().get::<Suppressed>(entity).is_some()
}

/// The current stance kind of `entity`.
pub(crate) fn stance_of(app: &App, entity: Entity) -> Option<StanceKind> {
    app.world().get::<Stance>(entity).map(|s| **s)
}

/// The current TU of `entity`.
pub(crate) fn tu_of(app: &App, entity: Entity) -> Option<u8> {
    app.world().get::<Tu>(entity).map(|t| **t)
}

/// The `single`-mode `FireModeSpec` of `shooter` (resolved off its wielded RANGED weapon),
/// so a test can emit a real `FireRequested`. NOT the canonical `test_support::single_mode`
/// fixture — this READS the mode the setup armed the shooter with (a resolver, not a builder).
pub(crate) fn wielded_single_mode(
    app: &mut App,
    shooter: Entity,
) -> gdtf_battle_sim::weapon::FireModeSpec {
    use bevy::ecs::relationship::RelationshipTarget as _;
    // Resolve shooter → the wielded entity carrying a FireMode (the ranged weapon; the melee
    // `fists` entity has none) → its single spec. A hand-rolled traversal via the world (the
    // test-body idiom).
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

/// Step `app` a fixed number of ticks.
pub(crate) fn step(app: &mut App, ticks: u32) {
    for _ in 0..ticks {
        app.update();
    }
}

/// Whether the active faction is currently `faction`'s.
pub(crate) fn active_faction_is(app: &App, faction: u8) -> bool {
    app.world()
        .get_resource::<gdtf_battle_sim::ActiveFaction>()
        .is_some_and(|active| ***active == faction)
}
