//! GTW-525 C3 — the `shove` WEAPON-TAG auto-shove, proven END-TO-END on the REAL
//! `setup_battle_on_request` -> `BattleSimPlugin` `Simulate`-band path (the gtw507 idiom):
//!
//! - **QA(6)** a `shove`-tagged MELEE weapon KNOCKS the target back one cell on a CONNECTING
//!   strike (in addition to the damage).
//! - **QA(7)** a `shove`-tagged RANGED weapon KNOCKS the target back on a CONNECTING shot.
//! - **QA(8a)** a NON-`shove` MELEE weapon never shoves (the same connecting strike leaves the
//!   target's cell UNCHANGED).
//! - **QA(8b)** a MISSED melee strike (the opposed roll lost) never shoves — even with a
//!   `shove`-tagged weapon.
//! - **QA(8c)** a NON-`shove` RANGED weapon never shoves on a CONNECTING shot — the negative
//!   discriminator for the fire-site tag-read (`weapon_shoves`): the shot connects (the same
//!   point-blank geometry as QA(7)) but the untagged gun leaves the target's cell UNCHANGED.
//! - **QA(8d)** a MISSED shot (the round strikes an empty lane, never a ganger) never shoves —
//!   even with a `shove`-tagged gun: the negative discriminator for the fire-site connect-read
//!   (`struck_ganger`).
//!
//! The auto-shove is bundled into the attack (no extra input): the connecting melee strike
//! (`dispatch_melee`) / ranged shot (`dispatch_fire`) writes an internal `ShoveRequested`
//! (`ShoveSource::Weapon`) that `dispatch_shove` (ordered `.after` both) resolves the SAME
//! frame through the SHARED shove verb. No pinned tunable magnitude — the assert is the
//! knock-back RELATION (the target's cell moved / did not move), never a balance number.
//!
//! QA(8a)/(8b) discriminate the MELEE connect-hook's tag-read + miss-gate; QA(8c)/(8d)
//! discriminate the SEPARATE RANGED connect-hook — `dispatch_fire`'s `weapon_shoves` tag-guard
//! and its `struck_ganger` connect-gate. Each fire-site clause has its own negative, so
//! dropping either half of the `(weapon_shoves, struck_ganger)` guard fails a test.

use bevy::{
    app::App,
    asset::AssetPlugin,
    prelude::{Entity, MinimalPlugins},
    scene::ScenePlugin,
};
use gdtf_battle_sim::{
    Cell, CellLevel, Cool, DamageType, Faction, FatalBias, FightMode, FightModeKind, FightModeSpec,
    Grit, Handedness, Position, Reach, Shove, Speed, Stance, StanceKind, Strength, Toughness,
    acts::MeleeRequested,
    battle::{BattleSimPlugin, SetupBattleRequested},
    ganger::{Direction, Facing, GangRegistry},
    magazine::{Magazine, ReloadTu},
    rng::BattleSeed,
    situation::{GangerSpawn, Situation},
    test_support::{
        GangerSpawnBuilder, SituationBuilder, test_armor_registry, test_terrain_registry,
    },
    tuning::{CombatTuning, ViewRange},
    weapon::{
        Accuracy, BaseSpread, FISTS_KEY, FireMode, FireModeSpec, Kickback, MagazineSize,
        MeleeWeaponRegistry, MeleeWeaponSpec, ModeConeMult, ModeKind, ModeShots, ModeTuPercent,
        WeaponDamage, WeaponName, WeaponPunch, WeaponRegistry, WeaponShred, WeaponSpec,
    },
};

/// An arbitrary seed for the test battle's RNG streams (determinism is asserted elsewhere).
const SEED: u64 = 0x5E1E_5405;
/// Gang 0 = player; gang 1 = enemy.
const PLAYER: u8 = 0;
const ENEMY: u8 = 1;
/// A view range comfortably covering an 8-adjacent strike / a point-blank shot.
const TEST_VIEW_RANGE: u16 = 12;

/// Level 0 — every fixture here is ground-floor, so the shove destination is always supported.
const fn level0() -> gdtf_battle_sim::Level {
    gdtf_battle_sim::Level::new(0)
}

/// A ground-floor `(cell, level)` key.
fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), level0())
}

/// A melee weapon spec with the chosen `shove` tag — arbitrary (not shipped) magnitudes big
/// enough to penetrate the test armor on a forced connect.
fn melee_spec(shove: bool) -> MeleeWeaponSpec {
    MeleeWeaponSpec {
        damage:      WeaponDamage::new(9),
        punch:       WeaponPunch::new(9),
        shred:       WeaponShred::new(2),
        damage_type: DamageType::Rend,
        fatal_bias:  FatalBias::new(4.0),
        handedness:  Handedness::OneHanded,
        reach:       Reach::new(1),
        fight_mode:  FightMode::new(vec![FightModeSpec::new(
            FightModeKind::Swing,
            gdtf_battle_sim::weapon::TuCost::new(20),
            gdtf_battle_sim::weapon::Strikes::new(1),
        )]),
        shove:       Shove::new(shove),
    }
}

/// A ranged weapon spec with the chosen `shove` tag — arbitrary magnitudes; a single-shot mode
/// so the point-blank shot resolves ONE connecting round.
fn ranged_spec(shove: bool) -> WeaponSpec {
    WeaponSpec {
        base_spread:      BaseSpread::new(0.01),
        accuracy:         Accuracy::new(5.0),
        kickback:         Kickback::new(0.0),
        fatal_bias:       FatalBias::new(7.0),
        damage:           WeaponDamage::new(12),
        punch:            WeaponPunch::new(20),
        shred:            WeaponShred::new(3),
        damage_type:      DamageType::Kinetic,
        magazine:         Magazine::loaded(MagazineSize::new(30), ReloadTu::new(12)),
        fire_mode:        FireMode::new(vec![FireModeSpec::new(
            ModeKind::Single,
            ModeConeMult::new(1.0),
            ModeTuPercent::new(0.2),
            ModeShots::new(1),
        )]),
        stable:           gdtf_battle_sim::weapon::Stable::new(false),
        shove:            Shove::new(shove),
        handedness:       Handedness::OneHanded,
        attachment_slots: Vec::new(),
    }
}

/// A melee registry whose `fists` default carries the chosen `shove` tag — EVERY setup-spawned
/// ganger (which authors no melee weapon → resolves `fists`) then wields it.
fn melee_registry(shove: bool) -> MeleeWeaponRegistry {
    MeleeWeaponRegistry::new([(WeaponName::new(FISTS_KEY.to_owned()), melee_spec(shove))])
}

/// A ranged registry whose `test-weapon` key carries the chosen `shove` tag.
fn ranged_registry(shove: bool) -> WeaponRegistry {
    WeaponRegistry::new([(
        WeaponName::new("test-weapon".to_owned()),
        ranged_spec(shove),
    )])
}

/// Build the full live-runtime harness (the gtw507 `battle_app` idiom) with the chosen
/// shove-tagged melee + ranged registries + the default melee tuning (variance > 0 for a valid
/// opposed roll).
fn battle_app(melee_shove: bool, ranged_shove: bool) -> App {
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
fn drive_setup(app: &mut App, situation_and_gangs: (Situation, GangRegistry)) {
    let (situation, gangs) = situation_and_gangs;
    app.world_mut().insert_resource(gangs);
    app.world_mut()
        .write_message(SetupBattleRequested::new(situation, BattleSeed::new(SEED)));
    for _ in 0..4 {
        app.update();
    }
}

/// The entity of the (sole) ganger of `faction`.
fn ganger_of(app: &mut App, faction: u8) -> Option<Entity> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Faction)>();
    query
        .iter(world)
        .find(|(_, f)| ***f == faction)
        .map(|(entity, _)| entity)
}

/// The current `(cell, level)` of `entity`.
fn pos_of(app: &App, entity: Entity) -> Option<CellLevel> {
    app.world().get::<Position>(entity).map(|p| **p)
}

/// A strong-Fight attacker (forced melee connect vs a zero-Fight defender under variance>0 via
/// the §7 degenerate `def ≤ 0` path).
fn strong_attacker(at: CellLevel, faction: u8, facing: Direction) -> GangerSpawn {
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
fn defenceless_target(at: CellLevel, faction: u8) -> GangerSpawn {
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
fn fighting_target(at: CellLevel, faction: u8) -> GangerSpawn {
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
fn weak_attacker(at: CellLevel, faction: u8, facing: Direction) -> GangerSpawn {
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
fn step(app: &mut App, ticks: u32) {
    for _ in 0..ticks {
        app.update();
    }
}

// ── QA(6) — a shove-tagged MELEE weapon knocks back on a connecting strike ─────

#[test]
fn shove_tagged_melee_connect_knocks_target_back() {
    // A shove-tagged fists (melee); ranged untagged (irrelevant — this is a melee strike).
    let mut app = battle_app(true, false);
    // Player attacker faces East at (5,5); defenceless enemy 8-adjacent East at (6,5); the
    // knock-back cell (7,5) is open ground (a supported lateral push).
    let situation = SituationBuilder::new()
        .with_gangers([
            strong_attacker(ground(5, 5), PLAYER, Direction::East),
            defenceless_target(ground(6, 5), ENEMY),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);
    let (Some(attacker), Some(target)) = (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
    else {
        unreachable!("setup spawns one player + one enemy");
    };
    assert_eq!(
        pos_of(&app, target),
        Some(ground(6, 5)),
        "target starts at (6,5)"
    );

    app.world_mut()
        .write_message(MeleeRequested::new(attacker, target));
    step(&mut app, 4);

    // C3: the connecting strike auto-shoved the target one cell East (away from the attacker).
    assert_eq!(
        pos_of(&app, target),
        Some(ground(7, 5)),
        "a `shove`-tagged melee weapon knocks the target back one cell on a connecting strike"
    );
}

// ── QA(7) — a shove-tagged RANGED weapon knocks back on a connecting shot ──────

#[test]
fn shove_tagged_ranged_connect_knocks_target_back() {
    // A shove-tagged ranged weapon; melee untagged (irrelevant — this is a shot).
    let mut app = battle_app(false, true);
    // Player shooter faces East at (5,5); enemy point-blank East at (6,5) (a near-certain
    // connect with the tight-cone / high-accuracy test gun); the knock-back cell (7,5) is open.
    let situation = SituationBuilder::new()
        .with_gangers([
            strong_attacker(ground(5, 5), PLAYER, Direction::East),
            defenceless_target(ground(6, 5), ENEMY),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);
    let (Some(shooter), Some(target)) = (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
    else {
        unreachable!("setup spawns one player + one enemy");
    };
    assert_eq!(
        pos_of(&app, target),
        Some(ground(6, 5)),
        "target starts at (6,5)"
    );

    // Fire the ranged weapon at the enemy's cell (a single-shot, near-guaranteed point-blank
    // connect with the tight cone). dispatch_fire's connect hook writes the weapon-tag shove.
    app.world_mut()
        .write_message(gdtf_battle_sim::acts::FireRequested::new(
            shooter,
            single_shot_mode(),
            Cell::new(6, 5),
            level0(),
        ));
    step(&mut app, 4);

    assert_eq!(
        pos_of(&app, target),
        Some(ground(7, 5)),
        "a `shove`-tagged gun knocks the target back one cell on a connecting shot"
    );
}

/// A single-shot fire-mode spec matching the ranged test weapon's authored mode (the message
/// carries an OWNED `FireModeSpec` — the input seam's shape).
const fn single_shot_mode() -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.2),
        ModeShots::new(1),
    )
}

// ── QA(8a) — a NON-shove weapon never shoves ───────────────────────────────────

#[test]
fn non_shove_melee_connect_does_not_knock_back() {
    // Both weapons UNtagged — a connecting strike must NOT move the target.
    let mut app = battle_app(false, false);
    let situation = SituationBuilder::new()
        .with_gangers([
            strong_attacker(ground(5, 5), PLAYER, Direction::East),
            defenceless_target(ground(6, 5), ENEMY),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);
    let (Some(attacker), Some(target)) = (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
    else {
        unreachable!("setup spawns one player + one enemy");
    };

    app.world_mut()
        .write_message(MeleeRequested::new(attacker, target));
    step(&mut app, 4);

    assert_eq!(
        pos_of(&app, target),
        Some(ground(6, 5)),
        "a NON-`shove` weapon never knocks the target back (the target stays put on a connect)"
    );
}

// ── QA(8b) — a MISSED strike never shoves (even with a shove weapon) ───────────

#[test]
fn shove_tagged_melee_miss_does_not_knock_back() {
    // A shove-tagged melee weapon, but a ZERO-Fight attacker vs a Fight-positive defender → the
    // opposed roll is LOST (a miss) → NO connect → NO shove.
    let mut app = battle_app(true, false);
    let situation = SituationBuilder::new()
        .with_gangers([
            weak_attacker(ground(5, 5), PLAYER, Direction::East),
            fighting_target(ground(6, 5), ENEMY),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);
    let (Some(attacker), Some(target)) = (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
    else {
        unreachable!("setup spawns one player + one enemy");
    };

    app.world_mut()
        .write_message(MeleeRequested::new(attacker, target));
    step(&mut app, 4);

    assert_eq!(
        pos_of(&app, target),
        Some(ground(6, 5)),
        "a MISSED strike never shoves — even with a `shove`-tagged weapon (the connect gate held)"
    );
}

// ── QA(8c) — a NON-shove RANGED weapon never shoves on a connecting shot ────────

#[test]
fn non_shove_ranged_connect_does_not_knock_back() {
    // Both weapons UNtagged — the SAME point-blank connect geometry as QA(7), but the gun is
    // NOT `shove`-tagged, so the connecting round must NOT move the target. This is the
    // fire-site negative for `dispatch_fire`'s `weapon_shoves` tag-read: dropping that guard
    // (an unconditional shove on ANY connecting round) would move the target here and fail.
    let mut app = battle_app(false, false);
    let situation = SituationBuilder::new()
        .with_gangers([
            strong_attacker(ground(5, 5), PLAYER, Direction::East),
            defenceless_target(ground(6, 5), ENEMY),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);
    let (Some(shooter), Some(target)) = (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
    else {
        unreachable!("setup spawns one player + one enemy");
    };
    assert_eq!(
        pos_of(&app, target),
        Some(ground(6, 5)),
        "target starts at (6,5)"
    );

    // Fire the UNtagged ranged weapon at the enemy's cell — a near-guaranteed point-blank
    // connect (the same tight-cone shot as QA(7)); the connect happens, only the tag differs.
    app.world_mut()
        .write_message(gdtf_battle_sim::acts::FireRequested::new(
            shooter,
            single_shot_mode(),
            Cell::new(6, 5),
            level0(),
        ));
    step(&mut app, 4);

    assert_eq!(
        pos_of(&app, target),
        Some(ground(6, 5)),
        "a NON-`shove` gun never knocks the target back — even on a connecting shot"
    );
}

// ── QA(8d) — a MISSED shot never shoves (even with a shove-tagged gun) ──────────

#[test]
fn shove_tagged_ranged_miss_does_not_knock_back() {
    // A `shove`-tagged ranged weapon, aimed AT the target's cell — but a WALL is interposed
    // between shooter and target, so the round deterministically STOPS on the wall (a
    // `ShotKind::Cover`/`Wall` outcome) and never reaches the ganger → no `ShotKind::Ganger`
    // round → `struck_ganger` is None → NO shove. This is the fire-site negative for
    // `dispatch_fire`'s connect-read (`struck_ganger`): the ganger IS the aimed occupant, so a
    // regression that shoved the AIMED occupant (or shoved on any tagged fire regardless of a
    // connect) would move the target here and FAIL — only the connect gate keeps it still.
    let mut app = battle_app(false, true);
    // Shooter faces East at (5,5); a wall fills (6,5); the target stands beyond it at (7,5).
    // The East shot marches into the wall at (6,5) and stops — the target is aimed-at but
    // shielded (a deterministic geometric miss, no RNG-dependent cone dodge).
    let situation = SituationBuilder::new()
        .with_gangers([
            strong_attacker(ground(5, 5), PLAYER, Direction::East),
            defenceless_target(ground(7, 5), ENEMY),
        ])
        .wall_at(ground(6, 5))
        .build_with_gangs();
    drive_setup(&mut app, situation);
    let (Some(shooter), Some(target)) = (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
    else {
        unreachable!("setup spawns one player + one enemy");
    };
    assert_eq!(
        pos_of(&app, target),
        Some(ground(7, 5)),
        "target starts at (7,5), behind the wall at (6,5)"
    );

    // Fire straight East AT the target's cell (7,5) — in-arc (no turn); the round is stopped by
    // the wall at (6,5) before reaching the ganger, so no round connects with a ganger.
    app.world_mut()
        .write_message(gdtf_battle_sim::acts::FireRequested::new(
            shooter,
            single_shot_mode(),
            Cell::new(7, 5),
            level0(),
        ));
    step(&mut app, 4);

    assert_eq!(
        pos_of(&app, target),
        Some(ground(7, 5)),
        "a MISSED shot never shoves — even with a `shove`-tagged gun (the connect gate held)"
    );
}
