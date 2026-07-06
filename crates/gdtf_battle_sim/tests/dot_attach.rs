//! GTW-544 (DOT, child GTW-41e of GTW-41) — the damage-over-time ATTACH path on the LIVE
//! fire path: a shot from a DOT weapon (one carrying a [`DotProfile`]) that lands a hit which
//! PENETRATES armour attaches (or REFRESHES) a [`Dot`] on the struck ganger; a fully-soaked
//! hit attaches nothing; a non-DOT weapon attaches nothing — proven END-TO-END on the REAL
//! `setup_battle_on_request` → `BattleSimPlugin` `Simulate`-band path, driven THROUGH a
//! buffered `FireRequested` (the same message the input seam writes).
//!
//! The clause contract this covers:
//!
//! - **(a) a penetrating DOT shot ATTACHES a Dot**: a shot from a DOT weapon that penetrates
//!   armour attaches a [`Dot`] on the struck ganger (PIN-DISCRIMINATING — fails if the
//!   attach seam is unwired).
//! - **(b) a fully-soaked shot attaches NO Dot**: a DOT shot against a target whose armour
//!   fully soaks the hit (penetrating damage `0`) attaches NO [`Dot`] — even though HP may
//!   still bruise.
//! - **(e) refresh-not-stack**: a SECOND penetrating DOT hit RESETS the affliction's turns to
//!   the profile (does not accumulate to 2×turns).
//! - **(f) a non-DOT weapon attaches NOTHING** (identity): the SAME penetrating shot from a
//!   weapon with no `DotProfile` leaves the target with no [`Dot`].
//!
//! The per-round `tick_dot` drain (HP-decrement / removal / the DOT-kills gate) is covered by
//! the in-crate unit tests (`effects::dot::test`); this file owns the attach seam.
//!
//! HARNESS NOTE (the `aoe_hittype` idiom): the sim crate is the LOW crate, so it cannot dev-dep
//! `gdtf_test_utils` (a cycle). The established sim-crate battle-integration idiom drives
//! `setup_battle_on_request` via a `SetupBattleRequested` message against a `MinimalPlugins` +
//! `AssetPlugin` + `ScenePlugin` + `BattleSimPlugin` app — the EXACT production wiring. NO
//! pinned tunable magnitudes: the tests assert Dot-present / Dot-absent / turns-reset, never a
//! specific damage number.

use bevy::{
    app::App,
    asset::AssetPlugin,
    prelude::{Entity, MinimalPlugins},
    scene::ScenePlugin,
};
use gdtf_battle_sim::{
    acts::FireRequested,
    battle::{BattleSimPlugin, SetupBattleRequested},
    ganger::{
        Aim, Aiming, Cool, Direction, Facing, GangRegistry, Grit, Speed, Strength, Toughness,
    },
    metric::{Cell, CellLevel, Level},
    prelude::{Faction, Position, Stance, StanceKind},
    rng::BattleSeed,
    situation::{GangerSpawn, Situation},
    test_support::{
        GangerSpawnBuilder, SituationBuilder, TEST_WEAPON_KEY, single_mode, test_armor_registry,
        test_melee_weapon_registry, test_weapon_spec,
    },
    tuning::{CombatTuning, ViewRange},
    weapon::{
        Accuracy, BaseSpread, DamageType, Dot, DotDamage, DotProfile, DotTurns, FatalBias,
        FireMode, FireModeSpec, Kickback, ModeConeMult, ModeKind, ModeShots, ModeTuPercent, Stable,
        WeaponDamage, WeaponName, WeaponPunch, WeaponRegistry, WeaponShred, WeaponSpec,
    },
};

/// The single faction every fixture ganger belongs to — the shooter AND its target. One
/// faction (no opponents) means no setup-time AI / reaction fire perturbs the state (the
/// GTW-541 clean-baseline recipe); the shooter firing on its own teammate is faction-blind and
/// fine for the attach assertion.
const PLAYER: u8 = 0;

/// A view range comfortably covering the pair.
const TEST_VIEW_RANGE: u16 = 20;

/// The DOT profile the test weapon carries — 5 HP/turn for 3 turns (magnitudes are mechanism,
/// never pinned).
const DOT_PER_TURN: u16 = 5;
const DOT_TURNS: u8 = 3;

/// A ground-floor `(cell, level)` key.
fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// The DOT profile the ATTACH tests expect on a penetrating hit.
const fn dot_profile() -> DotProfile {
    DotProfile::new(
        DotDamage::new(DOT_PER_TURN),
        DamageType::Plasma,
        DotTurns::new(DOT_TURNS),
    )
}

/// A ranged weapon spec with a TIGHT cone (zero spread, `stable`, high accuracy) so the shot
/// flies straight down the central axis and stops on the aimed-at target (the impact cell is
/// the aim cell deterministically — the "collapse the cone to the central axis" recipe). High
/// punch/damage so a connect PENETRATES the (armorless) test armour. `dot` is the optional DOT
/// profile: `Some` for a DOT weapon, `None` for the identity non-DOT weapon.
fn dot_weapon_spec(dot: Option<DotProfile>) -> WeaponSpec {
    WeaponSpec {
        base_spread: BaseSpread::new(0.0),
        accuracy: Accuracy::new(5.0),
        kickback: Kickback::new(0.0),
        fatal_bias: FatalBias::new(0.0),
        damage: WeaponDamage::new(20),
        punch: WeaponPunch::new(30),
        damage_type: DamageType::Plasma,
        fire_mode: FireMode::new(vec![single_mode(0.2, 1)]),
        stable: Stable::new(true),
        dot,
        ..test_weapon_spec()
    }
}

/// A DOT weapon that CANNOT penetrate the test armour — `punch 0` + `damage 1` against the
/// `test_armor_registry` suit (protection 2), so `PenetratingDamage == 0` (a fully-soaked hit
/// that may still bruise HP via the floor). Same tight cone so the shot still CONNECTS as a
/// ganger hit — the connect is geometry, the soak is the per-hit formula.
fn soaked_dot_weapon_spec() -> WeaponSpec {
    WeaponSpec {
        damage: WeaponDamage::new(1),
        punch: WeaponPunch::new(0),
        shred: WeaponShred::new(0),
        ..dot_weapon_spec(Some(dot_profile()))
    }
}

/// A [`WeaponRegistry`] whose `test-weapon` key carries the chosen DOT profile (or none).
fn dot_registry(dot: Option<DotProfile>) -> WeaponRegistry {
    WeaponRegistry::new([(
        WeaponName::new(TEST_WEAPON_KEY.to_owned()),
        dot_weapon_spec(dot),
    )])
}

/// A [`WeaponRegistry`] whose `test-weapon` key is the [`soaked_dot_weapon_spec`] (a DOT weapon
/// that cannot penetrate).
fn soaked_registry() -> WeaponRegistry {
    WeaponRegistry::new([(
        WeaponName::new(TEST_WEAPON_KEY.to_owned()),
        soaked_dot_weapon_spec(),
    )])
}

/// Build the live-runtime harness (the `aoe_hittype` `battle_app` idiom) with `seed` and a weapon
/// carrying `dot`.
fn battle_app(seed: u64, dot: Option<DotProfile>) -> (App, u64) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(CombatTuning {
        view_range: ViewRange::new(TEST_VIEW_RANGE),
        ..Default::default()
    });
    app.insert_resource(dot_registry(dot));
    // A melee registry (the `fists` default) so each ganger's melee weapon resolves at setup.
    app.insert_resource(test_melee_weapon_registry());
    app.insert_resource(test_armor_registry());
    (app, seed)
}

/// Build the harness with the [`soaked_registry`] (a DOT weapon that cannot penetrate the
/// test armour), for the fully-soaked-attaches-nothing test.
fn soaked_battle_app(seed: u64) -> (App, u64) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(CombatTuning {
        view_range: ViewRange::new(TEST_VIEW_RANGE),
        ..Default::default()
    });
    app.insert_resource(soaked_registry());
    app.insert_resource(test_melee_weapon_registry());
    app.insert_resource(test_armor_registry());
    (app, seed)
}

/// Drive a setup through the REAL `setup_battle_on_request` Ok path and settle it.
fn drive_setup(app: &mut App, seed: u64, situation_and_gangs: (Situation, GangRegistry)) {
    let (situation, gangs) = situation_and_gangs;
    app.world_mut().insert_resource(gangs);
    app.world_mut()
        .write_message(SetupBattleRequested::new(situation, BattleSeed::new(seed)));
    for _ in 0..4 {
        app.update();
    }
}

/// A high-Aim shooter at `at` facing `facing` (a fielded player ganger with a full TU pool +
/// strong Aim so the tight-cone shot connects).
fn shooter(at: CellLevel, facing: Direction) -> GangerSpawn {
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

/// A standing target at `at` with a deep HP pool (so a penetrating hit does not necessarily
/// down it) — the attach victim. The test armour is armorless, so a high-punch shot penetrates.
fn target(at: CellLevel) -> GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(PLAYER))
        .facing(Facing::new(Direction::West))
        .stance(Stance::new(StanceKind::Standing))
        .speed(Speed::new(10.0))
        .grit(Grit::new(40.0))
        .cool(Cool::new(40.0))
        .toughness(Toughness::new(40.0))
        .build()
}

/// The ganger occupying `at` (the setup-published position), or `None`.
fn ganger_at(app: &mut App, at: CellLevel) -> Option<Entity> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Position)>();
    query
        .iter(world)
        .find(|(_, p)| ***p == at)
        .map(|(entity, _)| entity)
}

/// The struck ganger's active [`Dot`] (the affliction attached on a penetrating DOT hit), or
/// `None`.
fn dot_of(app: &App, entity: Entity) -> Option<Dot> {
    app.world().get::<Dot>(entity).copied()
}

/// Step `app` a fixed number of ticks so a written `FireRequested` dispatches + resolves +
/// the `apply_dot` boundary runs (`.after(dispatch_fire)`).
fn step(app: &mut App, ticks: u32) {
    for _ in 0..ticks {
        app.update();
    }
}

/// The single fire mode the test weapon offers (the mode `FireRequested` carries).
const fn fire_mode() -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.2),
        ModeShots::new(1),
    )
}

/// Set up a shooter at (5,5)→East and a target at (8,5), fire one shot, and return the
/// `(shooter, target)` entities so a test can read the target's DOT after the shot.
fn fire_one_shot(app: &mut App, seed: u64) -> (Entity, Entity) {
    let situation = SituationBuilder::new()
        .with_gangers([shooter(ground(5, 5), Direction::East), target(ground(8, 5))])
        .build_with_gangs();
    drive_setup(app, seed, situation);

    let (Some(shooter_e), Some(target_e)) =
        (ganger_at(app, ground(5, 5)), ganger_at(app, ground(8, 5)))
    else {
        unreachable!("setup spawns the shooter + target at distinct cells");
    };

    app.world_mut().write_message(FireRequested::new(
        shooter_e,
        fire_mode(),
        Cell::new(8, 5),
        Level::new(0),
    ));
    step(app, 3);
    (shooter_e, target_e)
}

// === (a) a penetrating DOT shot ATTACHES a Dot. ===

#[test]
fn a_penetrating_dot_shot_attaches_a_dot() {
    let (mut app, seed) = battle_app(0x5544_0A0A, Some(dot_profile()));
    let (_shooter, target_e) = fire_one_shot(&mut app, seed);

    let attached = dot_of(&app, target_e);
    assert_eq!(
        attached,
        Some(Dot::from_profile(dot_profile())),
        "a penetrating hit from a DOT weapon attaches a Dot built from the weapon's profile \
         (PIN-DISCRIMINATING: fails if the attach seam is unwired)",
    );
}

// === (f) a NON-DOT weapon attaches NOTHING (identity). ===

#[test]
fn a_non_dot_weapon_attaches_nothing() {
    // The SAME penetrating shot, but the weapon carries NO DotProfile.
    let (mut app, seed) = battle_app(0x5544_0F0F, None);
    let (_shooter, target_e) = fire_one_shot(&mut app, seed);

    assert!(
        dot_of(&app, target_e).is_none(),
        "a weapon with no DotProfile attaches no Dot — the identity property (a penetrating \
         hit still wounds, but seeds no lingering burn)",
    );
}

// === (b) a fully-soaked DOT shot attaches NO Dot (penetrating == 0). ===

#[test]
fn a_fully_soaked_dot_shot_attaches_no_dot() {
    // The DOT weapon has punch 0 + damage 1 vs the test armour (protection 2), so
    // PenetratingDamage == 0 — the hit is fully soaked (it may still bruise HP via the floor,
    // but nothing penetrated), so no Dot attaches.
    let (mut app, seed) = soaked_battle_app(0x5544_0B0B);
    let (_shooter, target_e) = fire_one_shot(&mut app, seed);

    assert!(
        dot_of(&app, target_e).is_none(),
        "a fully-soaked hit (PenetratingDamage == 0) attaches NO Dot — even though HP may \
         still bruise (the DOT gate is penetration, not HP loss)",
    );
}

// === (e) refresh-not-stack: a SECOND penetrating DOT hit RESETS turns (does not stack). ===

#[test]
fn a_second_penetrating_dot_hit_refreshes_not_stacks() {
    let (mut app, seed) = battle_app(0x5544_0E0E, Some(dot_profile()));

    // First shot attaches the DOT.
    let situation = SituationBuilder::new()
        .with_gangers([shooter(ground(5, 5), Direction::East), target(ground(8, 5))])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);
    let (Some(shooter_e), Some(target_e)) = (
        ganger_at(&mut app, ground(5, 5)),
        ganger_at(&mut app, ground(8, 5)),
    ) else {
        unreachable!("setup spawns the shooter + target");
    };

    app.world_mut().write_message(FireRequested::new(
        shooter_e,
        fire_mode(),
        Cell::new(8, 5),
        Level::new(0),
    ));
    step(&mut app, 3);
    let first = dot_of(&app, target_e);
    assert_eq!(
        first.map(|d| *d.remaining_turns),
        Some(DOT_TURNS),
        "the first penetrating DOT hit attaches the full profile turn count",
    );

    // Second shot on the SAME target REFRESHES the DOT — its turns RESET to the profile's
    // DOT_TURNS, NOT accumulate to 2×DOT_TURNS (refresh-not-stack). Fire again (the shooter has
    // ammo + TU; a fresh turn is not needed — the second FireRequested resolves immediately).
    app.world_mut().write_message(FireRequested::new(
        shooter_e,
        fire_mode(),
        Cell::new(8, 5),
        Level::new(0),
    ));
    step(&mut app, 3);
    let second = dot_of(&app, target_e);
    assert_eq!(
        second.map(|d| *d.remaining_turns),
        Some(DOT_TURNS),
        "refresh-not-stack: a second penetrating DOT hit RESETS the remaining turns to the \
         profile ({DOT_TURNS}), never the stacked {}",
        DOT_TURNS * 2,
    );
}
