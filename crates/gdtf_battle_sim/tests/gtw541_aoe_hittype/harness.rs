//! Shared GTW-541 `AoE` fixture: the `AoE` weapon registry, the live battle-app
//! driver, the combatant builders, and the damage accessors.

use bevy::{
    app::App,
    asset::AssetPlugin,
    prelude::{Entity, MinimalPlugins},
    scene::ScenePlugin,
};
use gdtf_battle_sim::{
    Cool, Faction, Grit, Hp, Position, Speed, Stance, StanceKind, Strength, Toughness, Wounds,
    battle::{BattleSimPlugin, SetupBattleRequested},
    ganger::{Aim, Aiming, Direction, Facing, GangRegistry},
    metric::{Cell, CellLevel, Level},
    rng::BattleSeed,
    situation::{GangerSpawn, Situation},
    test_support::{
        GangerSpawnBuilder, TEST_WEAPON_KEY, test_armor_registry, test_melee_weapon_registry,
        test_weapon_spec,
    },
    tuning::{CombatTuning, ViewRange},
    weapon::{
        Accuracy, BaseSpread, DamageType, FatalBias, FireMode, FireModeSpec, HitType, Kickback,
        ModeConeMult, ModeKind, ModeShots, ModeTuPercent, Stable, WeaponDamage, WeaponName,
        WeaponPunch, WeaponRegistry, WeaponSpec,
    },
};

/// The single faction every fixture ganger belongs to — the shooter AND its targets. One
/// faction (no opponents) means no setup-time AI / reaction fire corrupts the baselines,
/// and the blast striking teammates IS the faction-blind friendly-fire property.
pub(crate) const PLAYER: u8 = 0;

/// A view range comfortably covering the whole cluster.
pub(crate) const TEST_VIEW_RANGE: u16 = 20;

/// A ground-floor `(cell, level)` key.
pub(crate) fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// A ranged weapon spec with the chosen `AoE` [`HitType`] and a TIGHT cone (zero spread,
/// `stable`, high accuracy) so the shot flies straight down the central axis and stops on
/// the aimed-at target — the impact cell is the aim cell deterministically (the memory
/// "collapse the cone to the central axis" recipe). High punch/damage so a connect wounds
/// through the (armorless) test armor. Single-shot, so ONE round resolves the template.
pub(crate) fn aoe_weapon_spec(hit_type: HitType) -> WeaponSpec {
    WeaponSpec {
        base_spread: BaseSpread::new(0.0),
        accuracy: Accuracy::new(5.0),
        kickback: Kickback::new(0.0),
        fatal_bias: FatalBias::new(2.0),
        damage: WeaponDamage::new(20),
        punch: WeaponPunch::new(30),
        damage_type: DamageType::Blast,
        fire_mode: FireMode::new(vec![FireModeSpec::with_hit_type(
            ModeKind::Single,
            ModeConeMult::new(1.0),
            ModeTuPercent::new(0.2),
            ModeShots::new(1),
            hit_type,
        )]),
        stable: Stable::new(true),
        ..test_weapon_spec()
    }
}

/// A [`WeaponRegistry`] whose `test-weapon` key (every setup-spawned ganger resolves it)
/// carries the chosen `AoE` [`HitType`].
pub(crate) fn aoe_registry(hit_type: HitType) -> WeaponRegistry {
    WeaponRegistry::new([(
        WeaponName::new(TEST_WEAPON_KEY.to_owned()),
        aoe_weapon_spec(hit_type),
    )])
}

/// Build the full live-runtime harness (the gtw508 `battle_app` idiom) with `seed` for the
/// RNG streams and a weapon whose one fire mode carries `hit_type`.
pub(crate) fn battle_app(seed: u64, hit_type: HitType) -> (App, u64) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(CombatTuning {
        view_range: ViewRange::new(TEST_VIEW_RANGE),
        ..Default::default()
    });
    app.insert_resource(aoe_registry(hit_type));
    // A melee registry (the `fists` default) so each ganger's melee weapon resolves at
    // setup; irrelevant to firing but required by the shared setup path.
    app.insert_resource(test_melee_weapon_registry());
    app.insert_resource(test_armor_registry());
    (app, seed)
}

/// Drive a setup through the REAL `setup_battle_on_request` Ok path with `seed` and settle
/// it (the deferred `bsn!` ganger scenes materialize and occupancy publishes).
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

/// A high-Aim shooter at `at` facing `facing` — a fielded player ganger with a full TU
/// pool + strong Aim so the tight-cone shot connects.
pub(crate) fn shooter(at: CellLevel, facing: Direction) -> GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(PLAYER))
        .facing(Facing::new(facing))
        .stance(Stance::new(StanceKind::Standing))
        .aiming(Aiming::new(true))
        .speed(Speed::new(20.0))
        .aim(Aim::new(20.0))
        .strength(Strength::new(20.0))
        .grit(Grit::new(20.0))
        .cool(Cool::new(20.0))
        .toughness(Toughness::new(12.0))
        .build()
}

/// A standing target at `at` with a deep HP pool (so a hit damages it without necessarily
/// downing it) — a splash / primary victim.
///
/// EVERY fixture ganger here is the SAME [`PLAYER`] faction ON PURPOSE: with no opposing
/// faction there is no AI engagement + no reaction fire during the setup settle, so the
/// only thing that ever changes a target's HP is the player-driven blast under test (a
/// clean baseline). The blast being faction-blind (friendly fire — `docs/combat/resolution.md`
/// §2) means the shooter's OWN teammates are struck, which is exactly the friendly-fire
/// property the ticket asks the test to demonstrate.
pub(crate) fn target(at: CellLevel) -> GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(PLAYER))
        .facing(Facing::new(Direction::West))
        .stance(Stance::new(StanceKind::Standing))
        .speed(Speed::new(10.0))
        .grit(Grit::new(30.0))
        .cool(Cool::new(30.0))
        .toughness(Toughness::new(30.0))
        .build()
}

/// The ganger occupying `at` (the setup-published position), or `None`.
pub(crate) fn ganger_at(app: &mut App, at: CellLevel) -> Option<Entity> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Position)>();
    query
        .iter(world)
        // `***p` derefs `&Position` → Position → CellLevel; compare it to the CellLevel arg.
        .find(|(_, p)| ***p == at)
        .map(|(entity, _)| entity)
}

/// The current HP of `entity`.
pub(crate) fn hp_of(app: &App, entity: Entity) -> Option<u16> {
    app.world().get::<Hp>(entity).map(|h| **h)
}

/// The current Wounds of `entity`.
pub(crate) fn wounds_of(app: &App, entity: Entity) -> Option<u8> {
    app.world().get::<Wounds>(entity).map(|w| **w)
}

/// The single-mode fire spec authored on `entity`'s wielded weapon (the mode
/// `FireRequested` carries). Reads the weapon registry's authored mode off the resolved
/// `FireMode` component through the ganger's wielded weapon. To keep the test simple we
/// reconstruct the same spec the registry authored (a single Blast/Single/Line mode).
pub(crate) const fn fire_mode(hit_type: HitType) -> FireModeSpec {
    FireModeSpec::with_hit_type(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.2),
        ModeShots::new(1),
        hit_type,
    )
}

/// Step `app` a fixed number of ticks so a written `FireRequested` dispatches + resolves.
pub(crate) fn step(app: &mut App, ticks: u32) {
    for _ in 0..ticks {
        app.update();
    }
}

/// Whether `entity` is still a fielded ganger carrying its battle state (a sanity guard —
/// the shooter is never despawned by firing).
pub(crate) fn is_fielded(app: &App, entity: Entity) -> bool {
    app.world().get::<Hp>(entity).is_some()
}

/// A ganger's `(Hp, Wounds)` battle-state snapshot — the pair a damage-vs-unchanged
/// comparison reads. `Wounds` is a life pool where FILLED = remaining (emptying it is
/// death), so it DECREASES under damage — the same direction as `Hp`.
pub(crate) fn vitals(app: &App, entity: Entity) -> (Option<u16>, Option<u8>) {
    (hp_of(app, entity), wounds_of(app, entity))
}

/// Whether `after` shows LESS Hp or LESS Wounds than `before` — the reliable "took damage"
/// signal (both pools deplete under damage; a graze may move only one of them).
pub(crate) const fn took_damage(
    before: (Option<u16>, Option<u8>),
    after: (Option<u16>, Option<u8>),
) -> bool {
    matches!((before.0, after.0), (Some(b), Some(a)) if a < b)
        || matches!((before.1, after.1), (Some(b), Some(a)) if a < b)
}
