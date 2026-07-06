//! Shared GTW-525 weapon-tag fixture: the shove-tagged melee / ranged registries, the live
//! battle-app driver, the combatant builders, and the accessors.

use bevy::{
    app::App,
    asset::AssetPlugin,
    prelude::{Entity, MinimalPlugins},
    scene::ScenePlugin,
};
use gdtf_battle_sim::{
    battle::{BattleSimPlugin, SetupBattleRequested},
    ganger::{Cool, Direction, Facing, GangRegistry, Grit, Speed, Strength, Toughness},
    prelude::{Cell, CellLevel, Faction, Position, Stance, StanceKind},
    rng::BattleSeed,
    situation::{GangerSpawn, Situation},
    test_support::{
        GangerSpawnBuilder, single_mode, test_armor_registry, test_melee_weapon_spec,
        test_terrain_registry, test_weapon_spec,
    },
    tuning::{CombatTuning, ViewRange},
    weapon::{
        Accuracy, BaseSpread, FISTS_KEY, FireMode, Kickback, MeleeWeaponRegistry, MeleeWeaponSpec,
        Shove, WeaponName, WeaponPunch, WeaponRegistry, WeaponSpec,
    },
};

/// An arbitrary seed for the test battle's RNG streams (determinism is asserted elsewhere).
pub(crate) const SEED: u64 = 0x5E1E_5405;
/// Gang 0 = player; gang 1 = enemy.
pub(crate) const PLAYER: u8 = 0;
pub(crate) const ENEMY: u8 = 1;
/// A view range comfortably covering an 8-adjacent strike / a point-blank shot.
pub(crate) const TEST_VIEW_RANGE: u16 = 12;

/// Level 0 — every fixture here is ground-floor, so the shove destination is always supported.
pub(crate) const fn level0() -> gdtf_battle_sim::metric::Level {
    gdtf_battle_sim::metric::Level::new(0)
}

/// A ground-floor `(cell, level)` key.
pub(crate) fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), level0())
}

/// A melee weapon spec with the chosen `shove` tag — arbitrary (not shipped) magnitudes big
/// enough to penetrate the test armor on a forced connect.
pub(crate) fn melee_spec(shove: bool) -> MeleeWeaponSpec {
    MeleeWeaponSpec {
        // Punch big enough to penetrate the test armor on a forced connect.
        punch: WeaponPunch::new(9),
        shove: Shove::new(shove),
        ..test_melee_weapon_spec()
    }
}

/// A ranged weapon spec with the chosen `shove` tag — arbitrary magnitudes; a single-shot mode
/// so the point-blank shot resolves ONE connecting round.
pub(crate) fn ranged_spec(shove: bool) -> WeaponSpec {
    WeaponSpec {
        // A near-zero cone + high accuracy + zero kickback so the point-blank shot
        // resolves ONE connecting round; punch big enough to penetrate the test armor.
        base_spread: BaseSpread::new(0.01),
        accuracy: Accuracy::new(5.0),
        kickback: Kickback::new(0.0),
        punch: WeaponPunch::new(20),
        fire_mode: FireMode::new(vec![single_mode(0.2, 1)]),
        shove: Shove::new(shove),
        ..test_weapon_spec()
    }
}

/// A melee registry whose `fists` default carries the chosen `shove` tag — EVERY setup-spawned
/// ganger (which authors no melee weapon → resolves `fists`) then wields it.
pub(crate) fn melee_registry(shove: bool) -> MeleeWeaponRegistry {
    MeleeWeaponRegistry::new([(WeaponName::new(FISTS_KEY.to_owned()), melee_spec(shove))])
}

/// A ranged registry whose `test-weapon` key carries the chosen `shove` tag.
pub(crate) fn ranged_registry(shove: bool) -> WeaponRegistry {
    WeaponRegistry::new([(
        WeaponName::new("test-weapon".to_owned()),
        ranged_spec(shove),
    )])
}

/// Build the full live-runtime harness (the `melee_act` `battle_app` idiom) with the chosen
/// shove-tagged melee + ranged registries + the default melee tuning (variance > 0 for a valid
/// opposed roll).
pub(crate) fn battle_app(melee_shove: bool, ranged_shove: bool) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(CombatTuning {
        view_range: ViewRange::new(TEST_VIEW_RANGE),
        ..Default::default()
    });
    app.insert_resource(ranged_registry(ranged_shove));
    app.insert_resource(melee_registry(melee_shove));
    app.insert_resource(test_armor_registry());
    // The terrain registry so a setup that authors a wall (QA(8d)'s interposed occluder) can
    // resolve it; the terrain-free tests never author a piece, so it is inert for them.
    app.insert_resource(test_terrain_registry());
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

/// The entity of the (sole) ganger of `faction`.
pub(crate) fn ganger_of(app: &mut App, faction: u8) -> Option<Entity> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Faction)>();
    query
        .iter(world)
        .find(|(_, f)| ***f == faction)
        .map(|(entity, _)| entity)
}

/// The current `(cell, level)` of `entity`.
pub(crate) fn pos_of(app: &App, entity: Entity) -> Option<CellLevel> {
    app.world().get::<Position>(entity).map(|p| **p)
}

/// A strong-Fight attacker (forced melee connect vs a zero-Fight defender under variance>0 via
/// the §7 degenerate `def ≤ 0` path).
pub(crate) fn strong_attacker(at: CellLevel, faction: u8, facing: Direction) -> GangerSpawn {
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

/// A zero-Fight defenceless target (forced-connect melee defender) with moderate Toughness (a
/// real HP pool to lose without every hit grazing).
pub(crate) fn defenceless_target(at: CellLevel, faction: u8) -> GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(faction))
        .facing(Facing::new(Direction::West))
        .stance(Stance::new(StanceKind::Standing))
        .speed(Speed::new(0.0))
        .strength(Strength::new(0.0))
        .grit(Grit::new(0.0))
        .cool(Cool::new(0.0))
        .toughness(Toughness::new(12.0))
        .build()
}

/// A Fight-positive target so a ZERO-Fight attacker MISSES it under variance 0 (the miss case).
pub(crate) fn fighting_target(at: CellLevel, faction: u8) -> GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(faction))
        .facing(Facing::new(Direction::West))
        .stance(Stance::new(StanceKind::Standing))
        .speed(Speed::new(20.0))
        .strength(Strength::new(20.0))
        .grit(Grit::new(20.0))
        .cool(Cool::new(20.0))
        .toughness(Toughness::new(120.0))
        .build()
}

/// A ZERO-Fight attacker (a guaranteed melee MISS vs a Fight-positive defender under variance 0).
pub(crate) fn weak_attacker(at: CellLevel, faction: u8, facing: Direction) -> GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(faction))
        .facing(Facing::new(facing))
        .stance(Stance::new(StanceKind::Standing))
        .speed(Speed::new(0.0))
        .strength(Strength::new(0.0))
        .grit(Grit::new(0.0))
        .cool(Cool::new(0.0))
        .build()
}

/// Step `app` a fixed number of ticks so a written request dispatches + the same-frame shove
/// resolves (`dispatch_shove` is `.after(dispatch_melee)` / `.after(dispatch_fire)`).
pub(crate) fn step(app: &mut App, ticks: u32) {
    for _ in 0..ticks {
        app.update();
    }
}
