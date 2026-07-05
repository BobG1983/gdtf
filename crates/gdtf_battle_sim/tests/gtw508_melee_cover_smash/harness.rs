//! Shared GTW-508 cover-smash fixture: the live battle-app driver, the cover seeding, the
//! melee / destroyed log recorders, the attacker builder, and the cover accessors.

use bevy::{
    app::App,
    asset::AssetPlugin,
    prelude::{Entity, MinimalPlugins, Resource},
    scene::ScenePlugin,
};
use gdtf_battle_sim::{
    Cool, Faction, Grit, Speed, Stance, StanceKind, Strength, Toughness, Tu,
    acts::MeleeResolved,
    armor::{ArmorHardness, ArmorProtection},
    battle::{BattleSimPlugin, SetupBattleRequested},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    ganger::{Direction, Facing, GangRegistry},
    metric::{Cell, CellLevel, Level},
    occupancy_sync::CoverDestroyed,
    rng::BattleSeed,
    situation::{GangerSpawn, Situation},
    test_support::{
        GangerSpawnBuilder, test_armor_registry, test_melee_weapon_registry, test_weapon_registry,
    },
    tuning::{CombatTuning, ViewRange},
};

/// Gang `0` is the player attacker.
pub(crate) const PLAYER: u8 = 0;

/// A view range comfortably covering an adjacent smash (arbitrary test tuning).
pub(crate) const TEST_VIEW_RANGE: u16 = 12;

/// A ground-floor `(cell, level)` key.
pub(crate) fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// Build the FULL live-runtime harness (the gtw507 `battle_app` idiom) with `seed` for the RNG
/// streams and a `CombatTuning` carrying the view range + the DEFAULT melee tuning (its
/// `mult_max` is the FORK-4a structural multiplier), plus the persistent `Load` weapon/armor
/// registries a `MinimalPlugins` app has no `AssetServer` to load.
pub(crate) fn battle_app(seed: u64) -> (App, u64) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(CombatTuning {
        view_range: ViewRange::new(TEST_VIEW_RANGE),
        ..Default::default()
    });
    app.insert_resource(test_weapon_registry());
    // The melee registry (with the `fists` default) so the attacker's melee weapon resolves at
    // setup (a fixture ganger authors none → `fists`, a damage-9 / punch-3 melee spec).
    app.insert_resource(test_melee_weapon_registry());
    app.insert_resource(test_armor_registry());
    (app, seed)
}

/// Drive a setup through the REAL `setup_battle_on_request` Ok path with `seed` and settle it
/// (the deferred `bsn!` ganger scenes materialize and the first-run recompute fills the fog).
pub(crate) fn drive_setup(
    app: &mut App,
    seed: u64,
    situation_and_gangs: (Situation, GangRegistry),
) {
    let (situation, gangs) = situation_and_gangs;
    app.world_mut().insert_resource(gangs);
    app.world_mut()
        .write_message(SetupBattleRequested::new(situation, BattleSeed::new(seed)));
    for _ in 0..4 {
        app.update();
    }
}

/// The entity of the (sole) player ganger.
pub(crate) fn player_ganger(app: &mut App) -> Option<Entity> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Faction)>();
    query
        .iter(world)
        .find(|(_, f)| ***f == PLAYER)
        .map(|(entity, _)| entity)
}

/// The current TU of `entity`.
pub(crate) fn tu_of(app: &App, entity: Entity) -> Option<u8> {
    app.world().get::<Tu>(entity).map(|t| **t)
}

/// The current HP of the cover at `at` in the model ledger, or `None` if the cell has no entry.
pub(crate) fn cover_hp(app: &App, at: CellLevel) -> Option<u32> {
    app.world()
        .get_resource::<CoverLedger>()
        .and_then(|ledger| ledger.peek(&at))
        .map(|entry| *entry.current_hp)
}

/// Whether the cover at `at` is marked destroyed in the model ledger.
pub(crate) fn cover_destroyed_flag(app: &App, at: CellLevel) -> bool {
    app.world()
        .get_resource::<CoverLedger>()
        .and_then(|ledger| ledger.peek(&at))
        .is_some_and(|entry| *entry.destroyed)
}

// ── Test-local recorders (a MessageReader only sees the current+previous update). ──

/// Every `MeleeResolved` observed across the run.
#[derive(Resource, Default)]
pub(crate) struct MeleeLog {
    /// One entry per `MeleeResolved` emitted.
    hits: Vec<MeleeResolved>,
}

/// Every `CoverDestroyed` observed across the run.
#[derive(Resource, Default)]
pub(crate) struct DestroyedLog {
    /// One entry per `CoverDestroyed` emitted.
    hits: Vec<CoverDestroyed>,
}

/// Drain `MeleeResolved` into the recorder.
pub(crate) fn record_melee(
    mut resolved: bevy::prelude::MessageReader<MeleeResolved>,
    mut log: bevy::prelude::ResMut<MeleeLog>,
) {
    for hit in resolved.read() {
        log.hits.push(*hit);
    }
}

/// Drain `CoverDestroyed` into the recorder.
pub(crate) fn record_destroyed(
    mut destroyed: bevy::prelude::MessageReader<CoverDestroyed>,
    mut log: bevy::prelude::ResMut<DestroyedLog>,
) {
    for hit in destroyed.read() {
        log.hits.push(*hit);
    }
}

/// Add both recorders (after `BattleSimPlugin`, so the buffers exist).
pub(crate) fn with_logs(app: &mut App) {
    app.init_resource::<MeleeLog>();
    app.init_resource::<DestroyedLog>();
    app.add_systems(bevy::app::Update, (record_melee, record_destroyed));
}

/// How many `MeleeResolved` were emitted across the run.
pub(crate) fn melee_hits(app: &App) -> usize {
    app.world()
        .get_resource::<MeleeLog>()
        .map_or(0, |log| log.hits.len())
}

/// How many `CoverDestroyed` were emitted across the run.
pub(crate) fn destroyed_hits(app: &App) -> usize {
    app.world()
        .get_resource::<DestroyedLog>()
        .map_or(0, |log| log.hits.len())
}

/// An attacker with a strong Fight — irrelevant to the structural path (no opposed roll), but a
/// full-stat fielded ganger so the melee weapon + TU pool resolve normally.
pub(crate) fn attacker(at: CellLevel, faction: u8, facing: Direction) -> GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(faction))
        .facing(Facing::new(facing))
        .stance(Stance::new(StanceKind::Standing))
        .speed(Speed::new(20.0))
        .strength(Strength::new(20.0))
        .grit(Grit::new(20.0))
        .cool(Cool::new(20.0))
        .toughness(Toughness::new(12.0))
        .build()
}

/// Seed an intact ARMORLESS cover piece at `at` in the model ledger with `max_hp` HP — the
/// structure the smash tests hit. Armorless (protection / hardness 0) so a connecting smash
/// removes HP (the tests assert HP moved, never a pinned magnitude). The `CoverLedger` is the
/// model surface the smash spends; seeding it directly is the gtw507 test idiom.
pub(crate) fn seed_cover(app: &mut App, at: CellLevel, max_hp: u32) {
    let entry = CoverEntry::seeded(
        CoverHp::new(max_hp),
        HeightBand::Mid,
        ArmorProtection::new(0),
        ArmorHardness::new(0),
    );
    app.world_mut()
        .resource_mut::<CoverLedger>()
        .insert(at, entry);
}

/// Step `app` a fixed number of ticks so a written `MeleeRequested` dispatches + the recorders
/// capture any emitted signals.
pub(crate) fn step(app: &mut App, ticks: u32) {
    for _ in 0..ticks {
        app.update();
    }
}
