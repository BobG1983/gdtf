//! GTW-547 (on-death effects, child GTW-41g) — on the LIVE combat path: a ganger whose
//! wielded weapon authors an `OnDeathEffect::Explode` DIES to a shot and fans a GTW-541 `AoE`
//! blast at its body (adjacent gangers take damage); a piece of cover whose `TerrainDef`
//! authors an `OnDeathEffect::LeaveField` is DESTROYED by fire and leaves a GTW-545 field at
//! its cell. Both proven END-TO-END on the REAL `setup_battle_on_request` → `BattleSimPlugin`
//! `Simulate`-band path, driven THROUGH a buffered `FireRequested` (the same message the
//! input seam writes) — so the `resolve_on_death` resolver runs in its real schedule slot.
//!
//! The clause contract this covers:
//!
//! - **Explode fans an `AoE` blast; cells in radius take damage**: a ganger with an Explode
//!   on-death effect that dies to a shot damages an ADJACENT ganger (the headline test).
//!   PIN-DISCRIMINATING (fails if the death→effect bridge or the resolver is unwired).
//! - **`LeaveField` spawns the referenced field at the death location**: a cover tile with a
//!   `LeaveField` on-death effect that is destroyed by fire spawns the referenced field at that
//!   cell (the second headline test), persisting per GTW-545 rules.
//! - **Both variants are RON-authorable on weapons AND cover tiles**: the weapon spec's
//!   `on_death` field + the terrain def's `on_death` field carry the effects through setup.
//!
//! NO pinned tunable magnitudes: the tests assert HP-DECREASED / field-present — never a
//! specific number. HARNESS NOTE (the gtw541/507/508 idiom): the sim crate is the LOW crate,
//! so it drives `setup_battle_on_request` via `SetupBattleRequested` against a `MinimalPlugins`
//! + `AssetPlugin` + `ScenePlugin` + `BattleSimPlugin` app (the EXACT production wiring).

use bevy::{
    app::App,
    asset::AssetPlugin,
    prelude::{Entity, MinimalPlugins},
    scene::ScenePlugin,
};
use gdtf_battle_sim::{
    Cool, Faction, Grit, Hp, LifeState, Position, Speed, Stance, StanceKind, Strength, Toughness,
    Wounds,
    acts::{FireRequested, MeleeRequested},
    battle::{BattleSimPlugin, SetupBattleRequested},
    cover::CoverHp,
    fields::{
        FieldDamage, FieldDef, FieldDefRegistry, FieldDuration, FieldKey, FieldRegistry,
        FieldTurns, ImmuneArmorTypes,
    },
    ganger::{Aim, Aiming, Direction, Facing, GangRegistry},
    magazine::{Magazine, ReloadTu},
    metric::{Cell, CellLevel, Level},
    on_death::OnDeathEffect,
    rng::BattleSeed,
    situation::{CoverSpawn, GangerSpawn, Situation},
    terrain::def::{
        TerrainDef, TerrainDefRegistry, TerrainDisplayName, TerrainPresenterKind, TerrainSimKind,
        TerrainUuid,
    },
    test_support::{
        GangerSpawnBuilder, SituationBuilder, TEST_MELEE_WEAPON_KEY, TEST_WEAPON_KEY,
        test_armor_registry, test_melee_weapon_registry,
    },
    tuning::{CombatTuning, ViewRange},
    weapon::{
        Accuracy, BaseSpread, BlastRadius, DamageType, FatalBias, FightMode, FightModeKind,
        FightModeSpec, FireMode, FireModeSpec, Handedness, HitType, Kickback, MagazineSize,
        MeleeWeaponRegistry, MeleeWeaponSpec, ModeConeMult, ModeKind, ModeShots, ModeTuPercent,
        Reach, Shove, Stable, Strikes, TuCost, WeaponDamage, WeaponName, WeaponPunch,
        WeaponRegistry, WeaponShred, WeaponSpec,
    },
};

/// The single faction every fixture ganger belongs to (the gtw541 all-one-faction recipe — no
/// setup-time AI / reaction fire corrupts baselines; the blast striking teammates IS the
/// faction-blind friendly-fire property).
const PLAYER: u8 = 0;
/// A view range covering the whole cluster.
const TEST_VIEW_RANGE: u16 = 20;
/// A field def UUID for the on-death cover test (a `Cover` sim-kind carrying `on_death`).
const BARREL: TerrainUuid = TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0547_0547_0001));

/// A ground-floor `(cell, level)` key.
fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// A lethal, tight-cone single-fire weapon carrying an `Explode` on-death effect — a hit
/// connects straight down the axis (zero spread, `stable`, high accuracy) and a huge
/// damage/punch KILLS the struck ganger, whose death then fans the radius-1 blast.
fn explode_weapon_spec() -> WeaponSpec {
    WeaponSpec {
        base_spread:      BaseSpread::new(0.0),
        accuracy:         Accuracy::new(5.0),
        kickback:         Kickback::new(0.0),
        fatal_bias:       FatalBias::new(50.0),
        damage:           WeaponDamage::new(500),
        punch:            WeaponPunch::new(500),
        shred:            WeaponShred::new(3),
        damage_type:      DamageType::Kinetic,
        magazine:         Magazine::loaded(MagazineSize::new(30), ReloadTu::new(12)),
        fire_mode:        FireMode::new(vec![FireModeSpec::new(
            ModeKind::Single,
            ModeConeMult::new(1.0),
            ModeTuPercent::new(0.2),
            ModeShots::new(1),
        )]),
        stable:           Stable::new(true),
        shove:            Shove::new(false),
        handedness:       Handedness::OneHanded,
        attachment_slots: Vec::new(),
        dot:              None,
        // GTW-547: the killed ganger detonates a radius-1 blast dealing a flat 50 HP per cell.
        on_death:         Some(OnDeathEffect::Explode {
            hit_type:    HitType::Blast {
                radius: BlastRadius::new(1),
            },
            damage:      gdtf_battle_sim::on_death::ExplodeDamage::new(50),
            damage_type: DamageType::Blast,
        }),
    }
}

/// A [`WeaponRegistry`] whose `test-weapon` key (every setup-spawned ganger resolves it)
/// carries the Explode on-death weapon.
fn explode_registry() -> WeaponRegistry {
    WeaponRegistry::new([(
        WeaponName::new(TEST_WEAPON_KEY.to_owned()),
        explode_weapon_spec(),
    )])
}

/// A terrain registry holding the standard test defs PLUS a `BARREL` cover def carrying a
/// `LeaveField` on-death effect (the referenced field is seeded into the field catalog).
fn barrel_terrain_registry() -> TerrainDefRegistry {
    let mut base = gdtf_battle_sim::test_support::test_terrain_registry();
    base.insert(
        BARREL,
        TerrainDef {
            key:            BARREL,
            display_name:   TerrainDisplayName::new("Fuel Barrel".to_owned()),
            sim_kind:       TerrainSimKind::Cover {
                hp:               CoverHp::new(1), // 1 HP so one shot destroys it
                armor_protection: gdtf_battle_sim::armor::ArmorProtection::new(0),
                armor_hardness:   gdtf_battle_sim::armor::ArmorHardness::new(0),
                height_band:      gdtf_battle_sim::cover::HeightBand::Low,
            },
            presenter_kind: TerrainPresenterKind::Cover {
                graphic_name: gdtf_battle_sim::terrain::piece::TerrainGraphicKey::new(
                    "cover".to_owned(),
                ),
            },
            tags:           Vec::new(),
            on_death:       Some(OnDeathEffect::LeaveField {
                field: FieldKey::new("burning".to_owned()),
            }),
        },
    );
    base
}

/// A field catalog holding the `burning` field the barrel leaves.
fn burning_field_registry() -> FieldDefRegistry {
    FieldDefRegistry::new([(
        FieldKey::new("burning".to_owned()),
        FieldDef::new(
            FieldDamage::new(3),
            DamageType::Plasma,
            ImmuneArmorTypes::default(),
            FieldDuration::Turns(FieldTurns::new(2)),
        ),
    )])
}

/// Build the live-runtime harness with `seed`, the Explode weapon, and (optionally) the barrel
/// terrain + field catalog for the `LeaveField` test.
fn battle_app(seed: u64, with_barrel: bool) -> (App, u64) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(CombatTuning {
        view_range: ViewRange::new(TEST_VIEW_RANGE),
        ..Default::default()
    });
    app.insert_resource(explode_registry());
    app.insert_resource(test_melee_weapon_registry());
    app.insert_resource(test_armor_registry());
    if with_barrel {
        app.insert_resource(barrel_terrain_registry());
        app.insert_resource(burning_field_registry());
    }
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

/// A high-Aim shooter at `at` facing `facing`.
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

/// A frail target at `at` — a shallow HP/Wounds pool so the huge Explode-weapon shot KILLS it
/// (flips it to Dead), triggering the on-death blast.
fn frail_target(at: CellLevel) -> GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(PLAYER))
        .facing(Facing::new(Direction::West))
        .stance(Stance::new(StanceKind::Standing))
        .speed(Speed::new(10.0))
        .grit(Grit::new(1.0))
        .cool(Cool::new(1.0))
        .toughness(Toughness::new(1.0))
        .build()
}

/// A sturdy bystander at `at` — a deep HP pool so it SURVIVES the on-death blast but visibly
/// LOSES HP to it (the "cells in radius take damage" assertion).
fn bystander(at: CellLevel) -> GangerSpawn {
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

/// The current Hp of `entity`.
fn hp_of(app: &App, entity: Entity) -> Option<u16> {
    app.world().get::<Hp>(entity).map(|h| **h)
}

/// The current Wounds of `entity`.
fn wounds_of(app: &App, entity: Entity) -> Option<u8> {
    app.world().get::<Wounds>(entity).map(|w| **w)
}

/// A ganger's `(Hp, Wounds)` snapshot.
fn vitals(app: &App, entity: Entity) -> (Option<u16>, Option<u8>) {
    (hp_of(app, entity), wounds_of(app, entity))
}

/// Whether `after` shows LESS Hp or LESS Wounds than `before` (both deplete under damage).
const fn took_damage(before: (Option<u16>, Option<u8>), after: (Option<u16>, Option<u8>)) -> bool {
    matches!((before.0, after.0), (Some(b), Some(a)) if a < b)
        || matches!((before.1, after.1), (Some(b), Some(a)) if a < b)
}

/// A plain single-target fire mode (the mode `FireRequested` carries).
const fn single_mode() -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.2),
        ModeShots::new(1),
    )
}

/// Step `app` a fixed number of ticks so a written `FireRequested` dispatches + resolves +
/// `resolve_on_death` fans the effect.
fn step(app: &mut App, ticks: u32) {
    for _ in 0..ticks {
        app.update();
    }
}

// === Explode headline: a killed ganger's on-death blast damages an adjacent ganger. ===

#[test]
fn a_killed_ganger_with_explode_on_death_damages_an_adjacent_ganger() {
    let (mut app, seed) = battle_app(0x5547_0A0A, false);

    // Shooter at (5,5) facing East; the FRAIL victim at (8,5) (the shot kills it); a sturdy
    // BYSTANDER at (8,4) — adjacent (north) to the victim, INSIDE the victim's death-blast
    // radius-1 disc. All one faction (the gtw541 clean-baseline recipe).
    let situation = SituationBuilder::new()
        .with_gangers([
            shooter(ground(5, 5), Direction::East),
            frail_target(ground(8, 5)),
            bystander(ground(8, 4)),
        ])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let (Some(shooter_e), Some(_victim), Some(neighbour)) = (
        ganger_at(&mut app, ground(5, 5)),
        ganger_at(&mut app, ground(8, 5)),
        ganger_at(&mut app, ground(8, 4)),
    ) else {
        unreachable!("setup spawns the shooter + victim + neighbour at distinct cells");
    };
    let neighbour0 = vitals(&app, neighbour);

    // Fire the lethal shot at the victim's cell. The kill emits OnDeathOccurred; the SAME-frame
    // (or next-tick) resolve_on_death fans the victim's Explode at (8,5), striking (8,4).
    app.world_mut().write_message(FireRequested::new(
        shooter_e,
        single_mode(),
        Cell::new(8, 5),
        Level::new(0),
    ));
    step(&mut app, 4);

    // The neighbour — never shot at directly — LOST HP to the victim's on-death blast.
    // PIN-DISCRIMINATING: with the death→effect bridge or resolve_on_death unwired the
    // neighbour is untouched (only the directly-shot victim would change).
    assert!(
        took_damage(neighbour0, vitals(&app, neighbour)),
        "the adjacent ganger took damage from the killed ganger's on-death Explode blast \
         (before {neighbour0:?}, after {:?})",
        vitals(&app, neighbour),
    );
}

// === `LeaveField` headline: destroyed cover leaves the referenced field at its cell. ===

#[test]
fn destroyed_cover_with_leave_field_spawns_the_field_at_that_cell() {
    let (mut app, seed) = battle_app(0x5547_0B0B, true);

    // Shooter at (5,5) facing East; a 1-HP fuel BARREL (Cover with a `LeaveField` on-death) at
    // (8,5). Aiming at the barrel cell destroys it in one shot; its destruction leaves the
    // `burning` field there.
    let situation = SituationBuilder::new()
        .with_gangers([shooter(ground(5, 5), Direction::East)])
        .with_scatter(CoverSpawn::new(ground(8, 5), BARREL))
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let Some(shooter_e) = ganger_at(&mut app, ground(5, 5)) else {
        unreachable!("setup spawns the shooter at (5,5)");
    };

    // Before the shot: no field at the barrel cell.
    assert!(
        !field_present(&app, ground(8, 5)),
        "no field exists at the barrel cell before it is destroyed"
    );

    // Fire at the barrel cell — the shot destroys the 1-HP cover, emitting CoverDestroyed +
    // OnDeathOccurred(cover); resolve_on_death then spawns the `burning` field there.
    app.world_mut().write_message(FireRequested::new(
        shooter_e,
        single_mode(),
        Cell::new(8, 5),
        Level::new(0),
    ));
    step(&mut app, 4);

    // The referenced field now exists at the destroyed barrel's cell (persists per GTW-545).
    assert!(
        field_present(&app, ground(8, 5)),
        "destroying the barrel left the `burning` field at its cell (the `LeaveField` effect)"
    );
}

/// Whether a live field sits at `at` in the battle-lifetime [`FieldRegistry`].
fn field_present(app: &App, at: CellLevel) -> bool {
    app.world()
        .get_resource::<FieldRegistry>()
        .is_some_and(|r| r.field_at(&at).is_some())
}

// === Melee ganger-kill gate: a melee-killed ganger fans its (ranged) on-death Explode. ===

/// A LETHAL melee weapon spec — a huge `Swing` blow so a forced connect KILLS the target
/// outright (flips it to Dead), triggering the melee terminal-death gate. Arbitrary magnitudes
/// (mechanism only, not shipped tuning).
fn lethal_melee_spec() -> MeleeWeaponSpec {
    MeleeWeaponSpec {
        damage:      WeaponDamage::new(500),
        punch:       WeaponPunch::new(500),
        shred:       WeaponShred::new(3),
        damage_type: DamageType::Rend,
        fatal_bias:  FatalBias::new(50.0),
        handedness:  Handedness::OneHanded,
        reach:       Reach::new(1),
        fight_mode:  FightMode::new(vec![FightModeSpec::new(
            FightModeKind::Swing,
            TuCost::new(20),
            Strikes::new(1),
        )]),
        shove:       Shove::new(false),
    }
}

/// A [`MeleeWeaponRegistry`] whose `fists` default (every fixture ganger resolves it, authoring
/// no melee weapon) is the LETHAL melee weapon — so a forced connect kills.
fn lethal_melee_registry() -> MeleeWeaponRegistry {
    MeleeWeaponRegistry::new([
        (
            WeaponName::new(gdtf_battle_sim::weapon::FISTS_KEY.to_owned()),
            lethal_melee_spec(),
        ),
        (
            WeaponName::new(TEST_MELEE_WEAPON_KEY.to_owned()),
            lethal_melee_spec(),
        ),
    ])
}

/// A strong melee attacker at `at` — high Fight contributors so a forced connect lands on the
/// defenceless victim (the gtw507 `strong_attacker` recipe).
fn melee_attacker(at: CellLevel, faction: u8, facing: Direction) -> GangerSpawn {
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

#[test]
fn a_ganger_killed_in_melee_fans_its_on_death_explode() {
    let (mut app, seed) = battle_app(0x5547_0C0C, false);
    // The lethal melee weapon so the strike KILLS the frail victim (the melee-kill gate).
    app.world_mut().insert_resource(lethal_melee_registry());

    // A faction-1 attacker at (5,5) facing East; the FRAIL victim (faction 0) 8-adjacent at
    // (6,5) — opposing faction + adjacent + clear LOS (the melee gates). A sturdy BYSTANDER
    // (faction 0) at (6,4) — adjacent (north) to the victim, INSIDE the victim's death-blast
    // radius-1 disc (the ranged Explode weapon rides every setup ganger).
    let situation = SituationBuilder::new()
        .with_gangers([
            melee_attacker(ground(5, 5), 1, Direction::East),
            frail_target(ground(6, 5)),
            bystander(ground(6, 4)),
        ])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let (Some(attacker), Some(victim), Some(neighbour)) = (
        ganger_at(&mut app, ground(5, 5)),
        ganger_at(&mut app, ground(6, 5)),
        ganger_at(&mut app, ground(6, 4)),
    ) else {
        unreachable!("setup spawns the attacker + victim + neighbour at distinct cells");
    };
    let neighbour0 = vitals(&app, neighbour);

    // Strike the victim in melee THROUGH the buffered MeleeRequested (the input-seam message).
    app.world_mut()
        .write_message(MeleeRequested::new(attacker, victim));
    step(&mut app, 4);

    // The victim died to the melee strike.
    assert_eq!(
        life_of(&app, victim),
        LifeState::Dead,
        "the lethal melee strike killed the victim"
    );
    // The neighbour — never struck directly — LOST HP to the melee-killed victim's on-death
    // Explode blast. PIN-DISCRIMINATING: reverting the melee ganger-kill `deaths.write(...)`
    // leaves the neighbour untouched (only the struck victim would change).
    assert!(
        took_damage(neighbour0, vitals(&app, neighbour)),
        "the adjacent ganger took damage from the melee-killed ganger's on-death Explode blast \
         (before {neighbour0:?}, after {:?})",
        vitals(&app, neighbour),
    );
}

/// The current `LifeState` of `entity` (Alive default if absent — no `unwrap`).
fn life_of(app: &App, entity: Entity) -> LifeState {
    app.world()
        .get::<LifeState>(entity)
        .copied()
        .unwrap_or(LifeState::Alive)
}

// === Melee cover-smash gate: a melee-smashed barrel leaves its on-death field. ===

#[test]
fn a_barrel_smashed_in_melee_leaves_its_on_death_field() {
    let (mut app, seed) = battle_app(0x5547_0D0D, true);
    // A lethal melee weapon so one smash destroys the 1-HP barrel (the cover-smash kill gate).
    app.world_mut().insert_resource(lethal_melee_registry());

    // A faction-0 attacker at (5,5) facing East; a 1-HP fuel BARREL (Cover with a `LeaveField`
    // on-death) at (6,5) — 8-adjacent, so the melee smash destroys it, leaving the field.
    let situation = SituationBuilder::new()
        .with_gangers([melee_attacker(ground(5, 5), PLAYER, Direction::East)])
        .with_scatter(CoverSpawn::new(ground(6, 5), BARREL))
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let Some(attacker) = ganger_at(&mut app, ground(5, 5)) else {
        unreachable!("setup spawns the attacker at (5,5)");
    };

    // Before the smash: no field at the barrel cell.
    assert!(
        !field_present(&app, ground(6, 5)),
        "no field exists at the barrel cell before it is smashed"
    );

    // Smash the barrel in melee THROUGH the buffered structural MeleeRequested. The smash
    // destroys the 1-HP cover, emitting OnDeathOccurred(cover); resolve_on_death then spawns
    // the `burning` field there.
    app.world_mut()
        .write_message(MeleeRequested::new_structural(attacker, ground(6, 5)));
    step(&mut app, 4);

    // The referenced field now exists at the smashed barrel's cell (the cover-smash gate).
    // PIN-DISCRIMINATING: reverting the melee cover-smash `deaths.write(...)` leaves no field.
    assert!(
        field_present(&app, ground(6, 5)),
        "smashing the barrel in melee left the `burning` field at its cell (the cover-smash \
         terminal-death gate)"
    );
}

// === Fire splash-kill gate: a Blast-splash-killed ganger fans its own on-death Explode. ===

/// A Blast-`HitType` fire spec on the Explode weapon — a `Single`-kind shot whose GTW-541
/// [`HitType::Blast`] template SPLASHES an `AoE`, letting the splash branch (not just the
/// primary round) exercise the death gate.
const fn blast_mode() -> FireModeSpec {
    FireModeSpec::with_hit_type(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.2),
        ModeShots::new(1),
        HitType::Blast {
            radius: BlastRadius::new(1),
        },
    )
}

#[test]
fn a_ganger_splash_killed_by_fire_fans_its_on_death_explode() {
    let (mut app, seed) = battle_app(0x5547_0E0E, false);

    // Shooter at (5,5) facing East. A sturdy ANCHOR at (8,5) — the aim cell — so the tight,
    // `stable` round impacts THERE (the gtw541 impact-is-aim-cell recipe: a ganger to stop
    // on), detonating the Blast at (8,5). A frail SPLASH-VICTIM at (8,4) — a splash occupant
    // in the radius-1 disc of (8,5), NOT the direct target — the blast splash KILLS it (the
    // splash-kill gate). A sturdy BYSTANDER at (8,3) — north of the splash-victim, inside ITS
    // OWN on-death blast radius but OUTSIDE the fired blast's radius-1 disc of (8,5). All one
    // faction (the friendly-fire clean-baseline recipe; the Blast is faction-blind).
    let situation = SituationBuilder::new()
        .with_gangers([
            shooter(ground(5, 5), Direction::East),
            bystander(ground(8, 5)),
            frail_target(ground(8, 4)),
            bystander(ground(8, 3)),
        ])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let (Some(shooter_e), Some(splash_victim), Some(neighbour)) = (
        ganger_at(&mut app, ground(5, 5)),
        ganger_at(&mut app, ground(8, 4)),
        ganger_at(&mut app, ground(8, 3)),
    ) else {
        unreachable!("setup spawns the shooter + splash-victim + neighbour at distinct cells");
    };
    let neighbour0 = vitals(&app, neighbour);

    // Fire a Blast aimed at (8,5): the round impacts the anchor there and detonates; the SPLASH
    // catches the frail off-axis victim at (8,4) and KILLS it; its on-death Explode then fans
    // onto the neighbour at (8,3).
    app.world_mut().write_message(FireRequested::new(
        shooter_e,
        blast_mode(),
        Cell::new(8, 5),
        Level::new(0),
    ));
    step(&mut app, 4);

    assert_eq!(
        life_of(&app, splash_victim),
        LifeState::Dead,
        "the Blast splash killed the off-axis victim"
    );
    // The neighbour at (8,3) — OUTSIDE the fired Blast's radius-1 disc of (8,5) — LOST HP only
    // to the splash-killed victim's OWN on-death Explode (whose radius-1 disc of (8,4) reaches
    // (8,3)). PIN-DISCRIMINATING: reverting the fire SPLASH-path `emit_on_death` leaves the
    // neighbour untouched (the splash kill would fan no effect).
    assert!(
        took_damage(neighbour0, vitals(&app, neighbour)),
        "the neighbour took damage from the splash-killed ganger's on-death Explode blast \
         (before {neighbour0:?}, after {:?})",
        vitals(&app, neighbour),
    );
}
