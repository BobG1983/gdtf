//! Shared GTW-507 melee-act fixture: the live battle-app driver, ganger accessors, the
//! `MeleeResolved` log machinery, and the shared combatant builders.

use bevy::{
    app::App,
    asset::AssetPlugin,
    prelude::{Entity, MinimalPlugins, Resource},
    scene::ScenePlugin,
};
use gdtf_battle_sim::{
    Cool, Faction, Grit, Hp, LifeState, Speed, Stance, StanceKind, Strength, Toughness, Tu, Wounds,
    acts::MeleeResolved,
    battle::{BattleSimPlugin, SetupBattleRequested},
    ganger::{Direction, Facing, GangRegistry},
    metric::{Cell, CellLevel, Level},
    rng::BattleSeed,
    situation::Situation,
    test_support::{
        GangerSpawnBuilder, test_armor_registry, test_melee_weapon_registry, test_weapon_registry,
    },
    tuning::{CombatTuning, ViewRange},
};

use super::connect::{BrokenLog, StruckLog, record_broken, record_struck};

/// An arbitrary (not shipped tuning) seed for the test battle's RNG streams.
pub(crate) const SEED: u64 = 0x5E1E_9907;

/// Gang `0` is the player; gang `1` is the enemy.
pub(crate) const PLAYER: u8 = 0;
pub(crate) const ENEMY: u8 = 1;

/// A view range comfortably covering an 8-adjacent strike (arbitrary test tuning).
pub(crate) const TEST_VIEW_RANGE: u16 = 12;

/// A ground-floor `(cell, level)` key.
pub(crate) fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// Build the FULL live-runtime harness (the gtw468 `battle_app` idiom) with a `CombatTuning`
/// carrying the view range + the DEFAULT melee tuning (its `variance > 0`, so `opposed_fight`'s
/// `random_range(1−v..1+v)` draw is a valid non-empty range), plus the persistent `Load`
/// weapon/armor registries a `MinimalPlugins` app has no `AssetServer` to load.
pub(crate) fn battle_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(CombatTuning {
        view_range: ViewRange::new(TEST_VIEW_RANGE),
        ..Default::default()
    });
    app.insert_resource(test_weapon_registry());
    // GTW-505: the melee registry (with the `fists` default) so each ganger's melee weapon
    // resolves at setup (fixture gangers author none → `fists`, which the test registry maps to
    // a damage-9 / punch-3 melee spec — guaranteeing a connecting hit penetrates the test armor).
    app.insert_resource(test_melee_weapon_registry());
    app.insert_resource(test_armor_registry());
    app
}

/// Drive a setup through the REAL `setup_battle_on_request` Ok path and settle it (the deferred
/// `bsn!` ganger scenes materialize and the first-run recompute fills the fog).
pub(crate) fn drive_setup(app: &mut App, situation_and_gangs: (Situation, GangRegistry)) {
    let (situation, gangs) = situation_and_gangs;
    app.world_mut().insert_resource(gangs);
    app.world_mut()
        .write_message(SetupBattleRequested::new(situation, BattleSeed::new(SEED)));
    for _ in 0..4 {
        app.update();
    }
}

/// The entity of the (sole) ganger of `faction`.
pub(crate) fn ganger_of(app: &mut App, faction: u8) -> Option<Entity> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Faction)>();
    query
        .iter(world)
        .find(|(_, f)| ***f == faction)
        .map(|(entity, _)| entity)
}

/// The current TU of `entity`.
pub(crate) fn tu_of(app: &App, entity: Entity) -> Option<u8> {
    app.world().get::<Tu>(entity).map(|t| **t)
}

/// The current HP of `entity`.
pub(crate) fn hp_of(app: &App, entity: Entity) -> Option<u16> {
    app.world().get::<Hp>(entity).map(|h| **h)
}

/// The current Wounds of `entity`.
pub(crate) fn wounds_of(app: &App, entity: Entity) -> Option<u8> {
    app.world().get::<Wounds>(entity).map(|w| **w)
}

/// The current `LifeState` of `entity`.
pub(crate) fn life_of(app: &App, entity: Entity) -> Option<LifeState> {
    app.world().get::<LifeState>(entity).copied()
}

// ── A test-local MeleeResolved recorder ─────────────────────────────────────────
//
// `dispatch_melee` emits a MeleeResolved per connecting hit; a `MessageReader` only drains a
// buffer once and only sees the current+previous update. To assert across the whole run we
// record every MeleeResolved into a resource the moment it is emitted.

/// Every `MeleeResolved` observed across the run — a test-only recorder so the assertion reads
/// the full history rather than racing the one-update message lifetime.
#[derive(Resource, Default)]
pub(crate) struct MeleeLog {
    /// One entry per `MeleeResolved` emitted (the struck cell + damage type carried).
    hits: Vec<MeleeResolved>,
}

/// Drain `MeleeResolved` into the recorder — added to the test app so the run's full strike
/// history is queryable after `app.update()`s.
pub(crate) fn record_melee(
    mut resolved: bevy::prelude::MessageReader<MeleeResolved>,
    mut log: bevy::prelude::ResMut<MeleeLog>,
) {
    for hit in resolved.read() {
        log.hits.push(*hit);
    }
}

/// Add the `MeleeResolved` + `MeleeStruck` + `ArmorBroken` recorders (after
/// `BattleSimPlugin`, so the buffers exist).
pub(crate) fn with_melee_log(app: &mut App) {
    app.init_resource::<MeleeLog>();
    app.init_resource::<StruckLog>();
    app.init_resource::<BrokenLog>();
    app.add_systems(
        bevy::app::Update,
        (record_melee, record_struck, record_broken),
    );
}

/// How many `MeleeResolved` were emitted across the run.
pub(crate) fn melee_hits(app: &App) -> usize {
    app.world()
        .get_resource::<MeleeLog>()
        .map_or(0, |log| log.hits.len())
}

/// An attacker ganger with a strong Fight (high Strength / Speed / Grit / Cool) so its rolled
/// Fight clearly beats a zero-Fight defender's (the forced-connect case under variance 0).
pub(crate) fn strong_attacker(
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
        .strength(Strength::new(20.0))
        .grit(Grit::new(20.0))
        .cool(Cool::new(20.0))
        .build()
}

/// A defenceless-in-melee target: ZERO Fight contributors (Strength / Speed / Grit / Cool all 0)
/// so a forced strike connects (the §7 degenerate `def ≤ 0` path), with a MODERATE Toughness
/// (`Hp = grit·Grit + toughness·Toughness + cool·Cool`) — enough HP that the strike's outcome is
/// observable, but low enough that the multiplied §6 penetrating damage clears the severity
/// floor so a WOUND (or a lethal down) registers, not a fully-mitigated graze.
pub(crate) fn defenceless_target(
    at: CellLevel,
    faction: u8,
) -> gdtf_battle_sim::situation::GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(faction))
        .facing(Facing::new(Direction::West))
        .stance(Stance::new(StanceKind::Standing))
        .speed(Speed::new(0.0))
        .strength(Strength::new(0.0))
        .grit(Grit::new(0.0))
        .cool(Cool::new(0.0))
        // Moderate Toughness: a real HP pool to lose, but not so much that the §6 score's
        // `−k·Toughness` mitigation pushes every connecting hit down to a graze.
        .toughness(Toughness::new(12.0))
        .build()
}

/// Step `app` a fixed number of ticks so a written `MeleeRequested` dispatches + the recorder
/// captures any `MeleeResolved`.
pub(crate) fn step(app: &mut App, ticks: u32) {
    for _ in 0..ticks {
        app.update();
    }
}
