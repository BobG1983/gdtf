//! GTW-541 (CORE of GTW-41) — `AoE` / `HitType` on the LIVE fire path: a weapon whose fired
//! mode carries a non-`Single` [`HitType`] applies its template at the shot's impact cell
//! and strikes EVERY occupant the template covers through the EXISTING
//! `resolve_and_apply` damage path — proven END-TO-END on the REAL
//! `setup_battle_on_request` → `BattleSimPlugin` `Simulate`-band path, driven THROUGH a
//! buffered `FireRequested` (the same message the input seam writes).
//!
//! The clause contract this covers:
//!
//! - **`AoE` damages every occupant in radius**: a weapon authored with `HitType::Blast`
//!   fired at an impact cell damages EVERY ganger in the blast radius (HP / Wounds change
//!   on MULTIPLE targets from ONE shot) — the direct target AND the splash occupants,
//!   faction-blind (friendly fire). PIN-DISCRIMINATING (fails if the splash is unwired).
//! - **`HitType::Single` is byte-identical**: a `Single` weapon fired at the same cluster
//!   damages ONLY the direct target — a bystander in an adjacent cell is untouched (no
//!   splash, the identity property).
//! - **Determinism**: the same `BattleSeed` reproduces an IDENTICAL multi-target outcome
//!   (the per-target HP after the blast is identical across two runs of the same seed).
//!
//! NO pinned tunable magnitudes: the tests assert HP-DECREASED / bystander-untouched /
//! seed-reproducibility — never a specific damage number.
//!
//! HARNESS NOTE (the gtw507/508 idiom): the sim crate is the LOW crate, so it cannot
//! dev-dep `gdtf_test_utils` (a cycle). The established sim-crate battle-integration idiom
//! drives `setup_battle_on_request` via a `SetupBattleRequested` message against a
//! `MinimalPlugins` + `AssetPlugin` + `ScenePlugin` + `BattleSimPlugin` app — the EXACT
//! production wiring.

use bevy::{
    app::App,
    asset::AssetPlugin,
    prelude::{Entity, MinimalPlugins},
    scene::ScenePlugin,
};
use gdtf_battle_sim::{
    Cool, Faction, Grit, Hp, Position, Speed, Stance, StanceKind, Strength, Toughness, Wounds,
    acts::FireRequested,
    battle::{BattleSimPlugin, SetupBattleRequested},
    ganger::{Aim, Aiming, Direction, Facing, GangRegistry},
    magazine::{Magazine, ReloadTu},
    metric::{Cell, CellLevel, Level},
    rng::BattleSeed,
    situation::{GangerSpawn, Situation},
    test_support::{
        GangerSpawnBuilder, SituationBuilder, TEST_WEAPON_KEY, test_armor_registry,
        test_melee_weapon_registry,
    },
    tuning::{CombatTuning, ViewRange},
    weapon::{
        Accuracy, BaseSpread, BlastRadius, DamageType, FatalBias, FireMode, FireModeSpec,
        Handedness, HitType, Kickback, MagazineSize, ModeConeMult, ModeKind, ModeShots,
        ModeTuPercent, Shove, Stable, WeaponDamage, WeaponName, WeaponPunch, WeaponRegistry,
        WeaponShred, WeaponSpec,
    },
};

/// The single faction every fixture ganger belongs to — the shooter AND its targets. One
/// faction (no opponents) means no setup-time AI / reaction fire corrupts the baselines,
/// and the blast striking teammates IS the faction-blind friendly-fire property.
const PLAYER: u8 = 0;

/// A view range comfortably covering the whole cluster.
const TEST_VIEW_RANGE: u16 = 20;

/// A ground-floor `(cell, level)` key.
fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// A ranged weapon spec with the chosen `AoE` [`HitType`] and a TIGHT cone (zero spread,
/// `stable`, high accuracy) so the shot flies straight down the central axis and stops on
/// the aimed-at target — the impact cell is the aim cell deterministically (the memory
/// "collapse the cone to the central axis" recipe). High punch/damage so a connect wounds
/// through the (armorless) test armor. Single-shot, so ONE round resolves the template.
fn aoe_weapon_spec(hit_type: HitType) -> WeaponSpec {
    WeaponSpec {
        base_spread:      BaseSpread::new(0.0),
        accuracy:         Accuracy::new(5.0),
        kickback:         Kickback::new(0.0),
        fatal_bias:       FatalBias::new(2.0),
        damage:           WeaponDamage::new(20),
        punch:            WeaponPunch::new(30),
        shred:            WeaponShred::new(3),
        damage_type:      DamageType::Blast,
        magazine:         Magazine::loaded(MagazineSize::new(30), ReloadTu::new(12)),
        fire_mode:        FireMode::new(vec![FireModeSpec::with_hit_type(
            ModeKind::Single,
            ModeConeMult::new(1.0),
            ModeTuPercent::new(0.2),
            ModeShots::new(1),
            hit_type,
        )]),
        stable:           Stable::new(true),
        shove:            Shove::new(false),
        handedness:       Handedness::OneHanded,
        attachment_slots: Vec::new(),
        dot:              None,
    }
}

/// A [`WeaponRegistry`] whose `test-weapon` key (every setup-spawned ganger resolves it)
/// carries the chosen `AoE` [`HitType`].
fn aoe_registry(hit_type: HitType) -> WeaponRegistry {
    WeaponRegistry::new([(
        WeaponName::new(TEST_WEAPON_KEY.to_owned()),
        aoe_weapon_spec(hit_type),
    )])
}

/// Build the full live-runtime harness (the gtw508 `battle_app` idiom) with `seed` for the
/// RNG streams and a weapon whose one fire mode carries `hit_type`.
fn battle_app(seed: u64, hit_type: HitType) -> (App, u64) {
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
fn drive_setup(app: &mut App, seed: u64, situation_and_gangs: (Situation, GangRegistry)) {
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

/// A standing target at `at` with a deep HP pool (so a hit damages it without necessarily
/// downing it) — a splash / primary victim.
///
/// EVERY fixture ganger here is the SAME [`PLAYER`] faction ON PURPOSE: with no opposing
/// faction there is no AI engagement + no reaction fire during the setup settle, so the
/// only thing that ever changes a target's HP is the player-driven blast under test (a
/// clean baseline). The blast being faction-blind (friendly fire — `docs/combat/resolution.md`
/// §2) means the shooter's OWN teammates are struck, which is exactly the friendly-fire
/// property the ticket asks the test to demonstrate.
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
        // `***p` derefs `&Position` → Position → CellLevel; compare it to the CellLevel arg.
        .find(|(_, p)| ***p == at)
        .map(|(entity, _)| entity)
}

/// The current HP of `entity`.
fn hp_of(app: &App, entity: Entity) -> Option<u16> {
    app.world().get::<Hp>(entity).map(|h| **h)
}

/// The current Wounds of `entity`.
fn wounds_of(app: &App, entity: Entity) -> Option<u8> {
    app.world().get::<Wounds>(entity).map(|w| **w)
}

/// The single-mode fire spec authored on `entity`'s wielded weapon (the mode
/// `FireRequested` carries). Reads the weapon registry's authored mode off the resolved
/// `FireMode` component through the ganger's wielded weapon. To keep the test simple we
/// reconstruct the same spec the registry authored (a single Blast/Single/Line mode).
const fn fire_mode(hit_type: HitType) -> FireModeSpec {
    FireModeSpec::with_hit_type(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.2),
        ModeShots::new(1),
        hit_type,
    )
}

/// Step `app` a fixed number of ticks so a written `FireRequested` dispatches + resolves.
fn step(app: &mut App, ticks: u32) {
    for _ in 0..ticks {
        app.update();
    }
}

/// Whether `entity` is still a fielded ganger carrying its battle state (a sanity guard —
/// the shooter is never despawned by firing).
fn is_fielded(app: &App, entity: Entity) -> bool {
    app.world().get::<Hp>(entity).is_some()
}

/// A ganger's `(Hp, Wounds)` battle-state snapshot — the pair a damage-vs-unchanged
/// comparison reads. `Wounds` is a life pool where FILLED = remaining (emptying it is
/// death), so it DECREASES under damage — the same direction as `Hp`.
fn vitals(app: &App, entity: Entity) -> (Option<u16>, Option<u8>) {
    (hp_of(app, entity), wounds_of(app, entity))
}

/// Whether `after` shows LESS Hp or LESS Wounds than `before` — the reliable "took damage"
/// signal (both pools deplete under damage; a graze may move only one of them).
const fn took_damage(before: (Option<u16>, Option<u8>), after: (Option<u16>, Option<u8>)) -> bool {
    matches!((before.0, after.0), (Some(b), Some(a)) if a < b)
        || matches!((before.1, after.1), (Some(b), Some(a)) if a < b)
}

// === AoE: a Blast weapon damages EVERY occupant in the radius (multiple targets, one shot). ===

#[test]
fn a_blast_shot_damages_every_occupant_in_the_radius() {
    let hit = HitType::Blast {
        radius: BlastRadius::new(1),
    };
    let (mut app, seed) = battle_app(0x5541_0A0A, hit);

    // Shooter at (5,5) facing East; the direct target at (8,5); two more clustered within
    // the radius-1 blast disc of the impact (8,5): a teammate at (8,4) (north) and one at
    // (9,5) (east). All the SAME faction (see `target` doc) — the blast striking them is
    // the faction-blind friendly-fire property.
    let situation = SituationBuilder::new()
        .with_gangers([
            shooter(ground(5, 5), Direction::East),
            target(ground(8, 5)),
            target(ground(8, 4)),
            target(ground(9, 5)),
        ])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let (Some(shooter_e), Some(direct), Some(splash_n), Some(splash_e)) = (
        ganger_at(&mut app, ground(5, 5)),
        ganger_at(&mut app, ground(8, 5)),
        ganger_at(&mut app, ground(8, 4)),
        ganger_at(&mut app, ground(9, 5)),
    ) else {
        unreachable!("setup spawns the shooter + three cluster targets at distinct cells");
    };
    assert_ne!(
        shooter_e, direct,
        "the shooter and the direct target are distinct"
    );

    let (direct0, north0, east0) = (
        vitals(&app, direct),
        vitals(&app, splash_n),
        vitals(&app, splash_e),
    );

    // Fire the blast at the direct target's cell.
    app.world_mut().write_message(FireRequested::new(
        shooter_e,
        fire_mode(hit),
        Cell::new(8, 5),
        Level::new(0),
    ));
    step(&mut app, 3);

    // The DIRECT target took damage.
    assert!(
        took_damage(direct0, vitals(&app, direct)),
        "the direct target is damaged by the blast (before {direct0:?}, after {:?})",
        vitals(&app, direct),
    );

    // The SPLASH occupants (a different cell than the impact) each took damage too — the
    // AoE struck MULTIPLE targets from ONE shot. PIN-DISCRIMINATING: with the splash
    // unwired only the direct target would be hurt.
    assert!(
        took_damage(north0, vitals(&app, splash_n)),
        "the northern splash occupant (8,4) is damaged by the radius-1 blast \
         (before {north0:?}, after {:?})",
        vitals(&app, splash_n),
    );
    assert!(
        took_damage(east0, vitals(&app, splash_e)),
        "the eastern splash occupant (9,5) is damaged by the radius-1 blast — the blast is \
         faction-blind (grenades do not discriminate) (before {east0:?}, after {:?})",
        vitals(&app, splash_e),
    );
    assert!(
        is_fielded(&app, shooter_e),
        "the shooter survives its own shot"
    );
}

// === Identity: a Single weapon damages ONLY the direct target — no splash. ===

#[test]
fn a_single_shot_damages_only_the_direct_target_no_splash() {
    let hit = HitType::Single;
    let (mut app, seed) = battle_app(0x5541_0B0B, hit);

    // The same cluster geometry as the blast test (all one faction), but the weapon is a
    // plain Single shot.
    let situation = SituationBuilder::new()
        .with_gangers([
            shooter(ground(5, 5), Direction::East),
            target(ground(8, 5)),
            target(ground(8, 4)),
            target(ground(9, 5)),
        ])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let (Some(shooter_e), Some(direct), Some(bystander_n), Some(bystander_e)) = (
        ganger_at(&mut app, ground(5, 5)),
        ganger_at(&mut app, ground(8, 5)),
        ganger_at(&mut app, ground(8, 4)),
        ganger_at(&mut app, ground(9, 5)),
    ) else {
        unreachable!("setup spawns the shooter + three cluster targets");
    };
    // Post-settle baselines (the all-one-faction cluster has no setup-time combat, so these
    // are clean; the shot under test is the only thing that can change them).
    let (direct0, north0, east0) = (
        vitals(&app, direct),
        vitals(&app, bystander_n),
        vitals(&app, bystander_e),
    );

    app.world_mut().write_message(FireRequested::new(
        shooter_e,
        fire_mode(hit),
        Cell::new(8, 5),
        Level::new(0),
    ));
    step(&mut app, 3);

    // The direct target was hit (a Single shot still works).
    assert!(
        took_damage(direct0, vitals(&app, direct)),
        "the Single shot damages its direct target (before {direct0:?}, after {:?})",
        vitals(&app, direct),
    );

    // The bystanders in adjacent cells are UNTOUCHED — no splash (the identity property):
    // their FULL vitals are unchanged from the post-settle baseline.
    assert_eq!(
        vitals(&app, bystander_n),
        north0,
        "a Single shot does NOT splash the northern bystander (no AoE — the identity property)",
    );
    assert_eq!(
        vitals(&app, bystander_e),
        east0,
        "a Single shot does NOT splash the eastern bystander (no AoE — the identity property)",
    );
}

// === Determinism: the same seed reproduces an identical multi-target blast outcome. ===

#[test]
fn the_blast_outcome_is_reproducible_under_the_same_seed() {
    let hit = HitType::Blast {
        radius: BlastRadius::new(1),
    };
    // Resolve the SAME blast under the SAME seed twice and read the per-target HP after.
    // A deterministic (sorted-order, single-stream) resolution reproduces byte-for-byte.
    let run = |seed: u64| -> Vec<Option<u16>> {
        let (mut app, seed) = battle_app(seed, hit);
        let situation = SituationBuilder::new()
            .with_gangers([
                shooter(ground(5, 5), Direction::East),
                target(ground(8, 5)),
                target(ground(8, 4)),
                target(ground(9, 5)),
            ])
            .build_with_gangs();
        drive_setup(&mut app, seed, situation);
        let Some(shooter_e) = ganger_at(&mut app, ground(5, 5)) else {
            unreachable!("setup spawns the shooter at (5,5)");
        };
        app.world_mut().write_message(FireRequested::new(
            shooter_e,
            fire_mode(hit),
            Cell::new(8, 5),
            Level::new(0),
        ));
        step(&mut app, 3);
        // The per-target HP after, read in a fixed cell order (deterministic readout).
        [ground(8, 5), ground(8, 4), ground(9, 5)]
            .into_iter()
            .map(|at| ganger_at(&mut app, at).and_then(|e| hp_of(&app, e)))
            .collect()
    };

    let a = run(0x5541_0C0C);
    let b = run(0x5541_0C0C);
    assert_eq!(
        a, b,
        "the same BattleSeed reproduces an identical multi-target blast outcome: {a:?} vs {b:?}",
    );
    // Non-vacuous: at least one target actually lost HP (the blast really landed).
    assert!(
        a.iter().flatten().any(|&hp| hp < 100),
        "precondition: the blast actually damaged the cluster (a real, non-vacuous outcome): {a:?}",
    );
}
