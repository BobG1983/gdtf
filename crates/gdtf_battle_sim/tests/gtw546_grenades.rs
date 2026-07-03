//! GTW-546 (child GTW-41d of GTW-41) — ARCED / LOBBED fire + grenades: a per-weapon
//! `TrajectoryStyle::Arc` grenade is THROWN along a deterministic parabola that clears
//! same-level cover, passes holes / windows, and is BLOCKED by an intact roof; on landing it
//! fans a GTW-541 `HitType::Blast` at the landing cell through the EXISTING
//! `resolve_and_apply` damage path. Proven END-TO-END on the REAL
//! `setup_battle_on_request` → `BattleSimPlugin` `Simulate`-band path, driven THROUGH a
//! buffered `ThrowGrenadeRequested` (the same message the input seam writes), plus pure
//! `march_arc` unit tests for the deterministic arc geometry.
//!
//! The clause contract this covers:
//!
//! - **`TrajectoryStyle` serde default `Straight`** — a weapon `.ron` that omits `trajectory:`
//!   parses as `Straight` (existing weapons unchanged); an authored `trajectory: Arc` parses
//!   as `Arc` (the identity property, the GTW-541 `HitType::Single` precedent).
//! - **Arc blocked by intact roof / passes holes** — `march_arc` from a higher thrower down
//!   onto a target under an intact `SlabState::Present` roof STOPS at the roof (a `Slab`
//!   landing at the roof cell, NOT the target); with a `SlabState::Destroyed` hole it PASSES
//!   and lands ON the target. Same-level lobs clear cover (never self-block on a same-level
//!   roof).
//! - **Blast hits room occupants through the roof hole** — a thrown Arc grenade lobbed at a
//!   room with a roof HOLE lands inside and its `HitType::Blast` DAMAGES the occupants (HP /
//!   Wounds drop). PIN-DISCRIMINATING (fails if the throw / blast is unwired).
//! - **Blast blocked by intact roof** — the SAME throw under an intact roof lands on the roof
//!   (a different cell / storey than the occupants), so the room occupants are UNTOUCHED.
//! - **Blind throw** — a thrower NOT FACING its target still resolves the throw (no LOS /
//!   facing / arc gate for an `Arc` weapon).
//! - **Determinism** — `march_arc` is a pure function (two identical calls agree); the arc
//!   geometry draws no RNG.
//!
//! NO pinned tunable magnitudes: the tests assert HP-DECREASED / untouched / landing-cell —
//! never a specific damage number.
//!
//! HARNESS NOTE (the gtw541 idiom): the sim crate is the LOW crate, so it cannot dev-dep
//! `gdtf_test_utils` (a cycle). The established sim-crate battle-integration idiom drives
//! `setup_battle_on_request` via a `SetupBattleRequested` message against a `MinimalPlugins`
//! + `AssetPlugin` + `ScenePlugin` + `BattleSimPlugin` app — the EXACT production wiring.

use bevy::{
    app::App,
    asset::AssetPlugin,
    prelude::{Entity, MinimalPlugins},
    scene::ScenePlugin,
};
use gdtf_battle_sim::{
    Cool, Faction, Grit, Hp, Position, Speed, Stance, StanceKind, Strength, Toughness, Wounds,
    acts::ThrowGrenadeRequested,
    battle::{BattleSimPlugin, SetupBattleRequested},
    ganger::{Aim, Direction, Facing, GangRegistry},
    magazine::{Magazine, ReloadTu},
    march::march_arc,
    metric::{Cell, CellLevel, Level},
    rng::BattleSeed,
    situation::{GangerSpawn, Situation},
    surface::{SlabState, SurfaceGrid},
    test_support::{
        GangerSpawnBuilder, SituationBuilder, TEST_WEAPON_KEY, test_armor_registry,
        test_melee_weapon_registry,
    },
    tuning::{CombatTuning, ViewRange},
    weapon::{
        Accuracy, BaseSpread, BlastRadius, DamageType, FatalBias, FireMode, FireModeSpec,
        Handedness, HitType, Kickback, MagazineSize, ModeConeMult, ModeKind, ModeShots,
        ModeTuPercent, Shove, Stable, TrajectoryStyle, WeaponDamage, WeaponName, WeaponPunch,
        WeaponRegistry, WeaponShred, WeaponSpec,
    },
};

/// The single faction every fixture ganger belongs to. One faction (no opponents) means no
/// setup-time AI / reaction fire corrupts the baselines, and the blast striking teammates IS
/// the faction-blind friendly-fire property (the gtw541 harness ruling).
const PLAYER: u8 = 0;

/// A view range comfortably covering the whole scene.
const TEST_VIEW_RANGE: u16 = 30;

/// A ground-floor `(cell, level)` key.
fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// An `(x, y)` key on a given storey.
fn at_level(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}

// === Pure `march_arc` geometry unit tests (deterministic, no app). ===

/// A tuning with the default projectile band edges — the arc march reads only the band
/// classification (for the reported band) + the slab surface; the default tuning suffices.
fn arc_tuning() -> CombatTuning {
    CombatTuning::default()
}

#[test]
fn arc_lands_on_the_target_through_open_sky() {
    // Thrower above (level 2) lobbing down onto a level-0 target, NO roof anywhere.
    let surface = SurfaceGrid::new();
    let landing = march_arc(at_level(5, 5, 2), ground(9, 5), &surface, &arc_tuning());
    // With no roof to intercept, the lob lands AT the target cell (cell + storey).
    assert_eq!(
        (landing.at.x, landing.at.y, landing.at.z),
        (9, 5, 0),
        "an unobstructed lob lands at the target cell: {landing:?}",
    );
}

#[test]
fn arc_is_blocked_by_an_intact_roof_between_the_thrower_and_a_lower_target() {
    // Thrower on level 1 lobbing DOWN onto a level-0 target under an INTACT roof at level 1
    // over the target column — the arc crosses the z=1 boundary and is stopped there.
    let mut surface = SurfaceGrid::new();
    surface.set_slab(at_level(6, 5, 1), SlabState::Present);
    let landing = march_arc(at_level(6, 5, 1), ground(6, 5), &surface, &arc_tuning());
    // The lob is stopped at the roof (level 1), NOT at the level-0 target — the target is
    // shielded (AC: arc blocked by intact roofs).
    assert_ne!(
        landing.at.z, 0,
        "an intact roof stops the lob above the target (not on the level-0 target): {landing:?}",
    );
    assert_eq!(
        landing.at.z, 1,
        "the lob lands on the intact roof at level 1: {landing:?}",
    );
}

#[test]
fn arc_passes_through_a_roof_hole_and_lands_on_the_lower_target() {
    // The SAME geometry as the blocked case, but the roof slab is DESTROYED (a hole) — the
    // lob drops through and lands on the level-0 target (AC: passes through holes / windows).
    let mut surface = SurfaceGrid::new();
    surface.set_slab(at_level(6, 5, 1), SlabState::Destroyed);
    let landing = march_arc(at_level(6, 5, 1), ground(6, 5), &surface, &arc_tuning());
    assert_eq!(
        (landing.at.x, landing.at.y, landing.at.z),
        (6, 5, 0),
        "a lob through a roof hole lands on the lower target: {landing:?}",
    );
}

#[test]
fn a_same_level_lob_is_not_self_blocked_by_a_same_level_roof() {
    // A short same-level throw under an intact roof at the storey ABOVE — the lob's apex stays
    // sub-storey, so it never crosses into the roofed level and lands on the target.
    let mut surface = SurfaceGrid::new();
    // A roof over the whole path at level 1.
    for x in 5..=9 {
        surface.set_slab(at_level(x, 5, 1), SlabState::Present);
    }
    let landing = march_arc(ground(5, 5), ground(9, 5), &surface, &arc_tuning());
    assert_eq!(
        (landing.at.x, landing.at.y, landing.at.z),
        (9, 5, 0),
        "a same-level lob clears cover but stays under the same-level roof, landing on target: {landing:?}",
    );
}

#[test]
fn march_arc_is_a_pure_function() {
    let mut surface = SurfaceGrid::new();
    surface.set_slab(at_level(6, 5, 1), SlabState::Destroyed);
    let a = march_arc(at_level(6, 5, 2), ground(9, 6), &surface, &arc_tuning());
    let b = march_arc(at_level(6, 5, 2), ground(9, 6), &surface, &arc_tuning());
    assert_eq!(a, b, "march_arc is deterministic (no RNG): {a:?} vs {b:?}");
}

// === TrajectoryStyle serde: default Straight, authored Arc. ===

#[test]
fn trajectory_defaults_to_straight_when_omitted() {
    // An existing-style weapon `.ron` with NO `trajectory:` field parses as Straight (the
    // identity property — every existing weapon is untouched).
    let ron = r"(
        base_spread: 0.1, accuracy: 1.0, kickback: 0.0, fatal_bias: 0.0,
        damage: 5, punch: 1, shred: 0, damage_type: Kinetic,
        magazine: (size: 6, reload_tu: 10),
        fire_mode: [(kind: Single, cone_mult: 1.0, tu_percent: 0.2, shots: 1)],
        stable: false, handedness: OneHanded,
    )";
    let Ok(spec) = ron::from_str::<WeaponSpec>(ron) else {
        unreachable!("the trajectory-less spec parses");
    };
    assert_eq!(
        spec.trajectory,
        TrajectoryStyle::Straight,
        "an omitted trajectory field defaults to Straight",
    );
}

#[test]
fn authored_arc_trajectory_parses() {
    let ron = r"(
        base_spread: 0.2, accuracy: 0.8, kickback: 0.0, fatal_bias: 0.0,
        damage: 8, punch: 2, shred: 1, damage_type: Blast,
        magazine: (size: 2, reload_tu: 18),
        trajectory: Arc,
        fire_mode: [(kind: Single, cone_mult: 1.0, tu_percent: 0.35, shots: 1, hit_type: Blast(radius: 1))],
        stable: false, handedness: OneHanded,
    )";
    let Ok(spec) = ron::from_str::<WeaponSpec>(ron) else {
        unreachable!("the arc grenade spec parses");
    };
    assert_eq!(
        spec.trajectory,
        TrajectoryStyle::Arc,
        "an authored `trajectory: Arc` parses as Arc",
    );
    assert!(spec.trajectory.is_arc(), "the arc weapon reports is_arc()");
}

// === Full-app throw dispatch: the blast damages room occupants (or is roof-blocked). ===

/// An ARC grenade spec: `trajectory: Arc`, one blast fire mode, high damage/punch so a
/// connect wounds through the (armorless) test armor. The FIRST `Single` mode carries the
/// blast the throw reads.
fn grenade_spec(radius: u8) -> WeaponSpec {
    WeaponSpec {
        base_spread: BaseSpread::new(0.2),
        accuracy:    Accuracy::new(0.8),
        kickback:    Kickback::new(0.0),
        fatal_bias:  FatalBias::new(2.0),
        damage:      WeaponDamage::new(20),
        punch:       WeaponPunch::new(30),
        shred:       WeaponShred::new(3),
        damage_type: DamageType::Blast,
        magazine:    Magazine::loaded(MagazineSize::new(4), ReloadTu::new(18)),
        fire_mode:   FireMode::new(vec![FireModeSpec::with_hit_type(
            ModeKind::Single,
            ModeConeMult::new(1.0),
            ModeTuPercent::new(0.35),
            ModeShots::new(1),
            HitType::Blast {
                radius: BlastRadius::new(radius),
            },
        )]),
        stable:      Stable::new(false),
        shove:       Shove::new(false),
        handedness:  Handedness::OneHanded,
        trajectory:  TrajectoryStyle::Arc,
        // GTW-554: a thrown grenade offers no attachment slots (the empty defaults).
        slots:       gdtf_battle_sim::WeaponSlots::default(),
        attachments: Vec::new(),
        dot:         None,
        on_death:    None,
    }
}

/// A [`WeaponRegistry`] whose `test-weapon` key (every setup-spawned ganger resolves it)
/// carries an Arc grenade with the chosen blast radius.
fn grenade_registry(radius: u8) -> WeaponRegistry {
    WeaponRegistry::new([(
        WeaponName::new(TEST_WEAPON_KEY.to_owned()),
        grenade_spec(radius),
    )])
}

/// Build the full live-runtime harness with `seed` + an Arc grenade of the given radius.
fn battle_app(seed: u64, radius: u8) -> (App, u64) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(CombatTuning {
        view_range: ViewRange::new(TEST_VIEW_RANGE),
        ..Default::default()
    });
    app.insert_resource(grenade_registry(radius));
    app.insert_resource(test_melee_weapon_registry());
    app.insert_resource(test_armor_registry());
    (app, seed)
}

/// Drive a setup through the REAL `setup_battle_on_request` Ok path with `seed` and settle it.
fn drive_setup(app: &mut App, seed: u64, situation_and_gangs: (Situation, GangRegistry)) {
    let (situation, gangs) = situation_and_gangs;
    app.world_mut().insert_resource(gangs);
    app.world_mut()
        .write_message(SetupBattleRequested::new(situation, BattleSeed::new(seed)));
    for _ in 0..4 {
        app.update();
    }
}

/// A thrower at `at` FACING `facing` — a fielded player ganger with a full TU pool.
fn thrower(at: CellLevel, facing: Direction) -> GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(PLAYER))
        .facing(Facing::new(facing))
        .stance(Stance::new(StanceKind::Standing))
        .speed(Speed::new(20.0))
        .aim(Aim::new(20.0))
        .strength(Strength::new(20.0))
        .grit(Grit::new(20.0))
        .cool(Cool::new(20.0))
        .toughness(Toughness::new(12.0))
        .build()
}

/// A standing target at `at` with a deep HP pool — a blast victim.
fn target(at: CellLevel) -> GangerSpawn {
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
fn ganger_at(app: &mut App, at: CellLevel) -> Option<Entity> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Position)>();
    query
        .iter(world)
        .find(|(_, p)| ***p == at)
        .map(|(entity, _)| entity)
}

/// The current `(Hp, Wounds)` of `entity`.
fn vitals(app: &App, entity: Entity) -> (Option<u16>, Option<u8>) {
    (
        app.world().get::<Hp>(entity).map(|h| **h),
        app.world().get::<Wounds>(entity).map(|w| **w),
    )
}

/// Whether `after` shows LESS Hp or LESS Wounds than `before` (both pools deplete under damage).
const fn took_damage(before: (Option<u16>, Option<u8>), after: (Option<u16>, Option<u8>)) -> bool {
    matches!((before.0, after.0), (Some(b), Some(a)) if a < b)
        || matches!((before.1, after.1), (Some(b), Some(a)) if a < b)
}

/// Author an intact roof (a `SlabState::Present` slab) at level 1 over the given cells (their
/// column's roof), mutating the setup-published `SurfaceGrid` in the test body.
fn set_roof(app: &mut App, cells: &[(i32, i32)], state: SlabState) {
    let mut surface = app.world_mut().resource_mut::<SurfaceGrid>();
    for &(x, y) in cells {
        surface.set_slab(at_level(x, y, 1), state);
    }
}

/// Step `app` a fixed number of ticks so a written `ThrowGrenadeRequested` dispatches + resolves.
fn step(app: &mut App, ticks: u32) {
    for _ in 0..ticks {
        app.update();
    }
}

#[test]
fn a_grenade_lobbed_through_a_roof_hole_damages_the_room_occupants() {
    let (mut app, seed) = battle_app(0x5546_0A0A, 1);
    // Thrower on level 1 (a rooftop / upper storey) at (5,5); a cluster of targets on level 0
    // inside the room below, around the target cell (5,7).
    let situation = SituationBuilder::new()
        .with_gangers([
            thrower(at_level(5, 5, 1), Direction::East),
            target(ground(5, 7)),
            target(ground(6, 7)),
        ])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    // A HOLE in the roof over the whole descent path onto the room — the lob drops through.
    set_roof(
        &mut app,
        &[(5, 5), (5, 6), (5, 7), (6, 6), (6, 7)],
        SlabState::Destroyed,
    );

    let (Some(thrower_e), Some(occ_a), Some(occ_b)) = (
        ganger_at(&mut app, at_level(5, 5, 1)),
        ganger_at(&mut app, ground(5, 7)),
        ganger_at(&mut app, ground(6, 7)),
    ) else {
        unreachable!("setup spawns the thrower + two room occupants");
    };
    let (a0, b0) = (vitals(&app, occ_a), vitals(&app, occ_b));

    // Lob the grenade at the room's target cell (5,7) on level 0.
    app.world_mut()
        .write_message(ThrowGrenadeRequested::new(thrower_e, ground(5, 7)));
    step(&mut app, 3);

    assert!(
        took_damage(a0, vitals(&app, occ_a)),
        "the direct room occupant (5,7) is damaged by the lobbed blast (before {a0:?}, after {:?})",
        vitals(&app, occ_a),
    );
    assert!(
        took_damage(b0, vitals(&app, occ_b)),
        "the adjacent room occupant (6,7) is caught by the radius-1 blast (before {b0:?}, after {:?})",
        vitals(&app, occ_b),
    );
}

#[test]
fn a_grenade_lobbed_at_an_intact_roof_is_blocked_and_spares_the_room() {
    let (mut app, seed) = battle_app(0x5546_0B0B, 1);
    // The SAME geometry as the hole case, but the roof is INTACT.
    let situation = SituationBuilder::new()
        .with_gangers([
            thrower(at_level(5, 5, 1), Direction::East),
            target(ground(5, 7)),
            target(ground(6, 7)),
        ])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    // An INTACT roof over the whole descent path — the lob is stopped on the roof (level 1),
    // sparing the level-0 room occupants.
    set_roof(
        &mut app,
        &[(5, 5), (5, 6), (5, 7), (6, 6), (6, 7)],
        SlabState::Present,
    );

    let (Some(thrower_e), Some(occ_a), Some(occ_b)) = (
        ganger_at(&mut app, at_level(5, 5, 1)),
        ganger_at(&mut app, ground(5, 7)),
        ganger_at(&mut app, ground(6, 7)),
    ) else {
        unreachable!("setup spawns the thrower + two room occupants");
    };
    let (a0, b0) = (vitals(&app, occ_a), vitals(&app, occ_b));

    app.world_mut()
        .write_message(ThrowGrenadeRequested::new(thrower_e, ground(5, 7)));
    step(&mut app, 3);

    // The intact roof blocked the lob at level 1 — the level-0 room occupants are UNTOUCHED
    // (the blast fanned at the roof cell, a different storey). PIN-DISCRIMINATING: with the
    // roof block unwired the grenade would drop through and damage them.
    assert_eq!(
        vitals(&app, occ_a),
        a0,
        "an intact roof spares the room occupant (5,7) — the lob is blocked above it",
    );
    assert_eq!(
        vitals(&app, occ_b),
        b0,
        "an intact roof spares the room occupant (6,7)",
    );
}

#[test]
fn a_blind_throw_resolves_without_a_facing_or_los_gate() {
    let (mut app, seed) = battle_app(0x5546_0C0C, 1);
    // The thrower FACES away (North) from the target to its East — a straight shot's facing-arc
    // gate would refuse it, but a blind lob has NO such gate.
    let situation = SituationBuilder::new()
        .with_gangers([
            thrower(ground(5, 5), Direction::North),
            target(ground(9, 5)),
        ])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let (Some(thrower_e), Some(victim)) = (
        ganger_at(&mut app, ground(5, 5)),
        ganger_at(&mut app, ground(9, 5)),
    ) else {
        unreachable!("setup spawns the thrower + the target");
    };
    let v0 = vitals(&app, victim);

    // Lob at the target cell despite the thrower facing away — no roof, so it lands on target.
    app.world_mut()
        .write_message(ThrowGrenadeRequested::new(thrower_e, ground(9, 5)));
    step(&mut app, 3);

    assert!(
        took_damage(v0, vitals(&app, victim)),
        "a blind lob (thrower facing away) still resolves and damages the target — no facing / \
         LOS gate for an Arc weapon (before {v0:?}, after {:?})",
        vitals(&app, victim),
    );
}
