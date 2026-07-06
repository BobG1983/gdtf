//! FIRE while manned — the occupied ganger fires the MOUNTED weapon (not its own gun),
//! deterministically, with the fire-only `ShotFired` log machinery colocated here.

use bevy::{
    app::App,
    prelude::{Entity, Resource},
};
use gdtf_battle_sim::{
    DamageType, Faction, FireModeSpec, ModeConeMult, ModeKind, ModeShots, ModeTuPercent, Stance,
    StanceKind, Toughness,
    acts::{EnterEmplacementRequested, FireRequested},
    ganger::{Direction, Facing},
    metric::{Cell, CellLevel, Level},
    shot_fired::ShotFired,
    situation::GangerSpawn,
    test_support::{GangerSpawnBuilder, SituationBuilder},
    weapon::WeaponName,
};

use super::harness::*;

/// Every [`ShotFired`] observed across the run (a `MessageReader` only sees the current+previous
/// update, so a recorder resource accumulates them).
#[derive(Resource, Default)]
struct ShotLog {
    /// One entry per `ShotFired` emitted.
    shots: Vec<ShotFired>,
}

/// Drain `ShotFired` into the recorder.
fn record_shots(
    mut fired: bevy::prelude::MessageReader<ShotFired>,
    mut log: bevy::prelude::ResMut<ShotLog>,
) {
    for shot in fired.read() {
        log.shots.push(shot.clone());
    }
}

/// Add the `ShotFired` recorder (after `BattleSimPlugin`, so the buffer exists).
fn with_shot_log(app: &mut App) {
    app.init_resource::<ShotLog>();
    app.add_systems(bevy::app::Update, record_shots);
}

/// The `DamageType` of the FIRST recorded `ShotFired`, if any (discriminates which gun fired).
fn first_shot_damage_type(app: &App) -> Option<DamageType> {
    app.world()
        .get_resource::<ShotLog>()
        .and_then(|log| log.shots.first().map(|s| s.damage))
}

// ── FIRE: the occupied ganger fires the MOUNTED weapon (not its own gun) ────────

/// An enemy ganger at `at` facing `facing` (gang 1) — a target for the mounted-gun fire test.
fn enemy_at(at: CellLevel, facing: Direction) -> GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(1))
        .facing(Facing::new(facing))
        .stance(Stance::new(StanceKind::Standing))
        .weapon(WeaponName::new(OWN_KEY.to_owned()))
        .toughness(Toughness::new(12.0))
        .build()
}

/// While OCCUPIED, the ganger's fire resolves the MOUNTED weapon: a `FireRequested` produces a
/// `ShotFired` carrying the MOUNTED gun's `DamageType` (distinct from the ganger's own gun), through
/// the real `dispatch_fire` → `fire()` path, and the outcome is seed-DETERMINISTIC (two same-seed
/// runs agree).
#[test]
fn occupied_ganger_fires_the_mounted_weapon_deterministically() {
    // Run the enter→fire flow and return the FIRST ShotFired's DamageType (the discriminator).
    let run = |seed: u64| -> Option<DamageType> {
        let (mut app, seed) = battle_app(seed);
        with_shot_log(&mut app);
        // Player at (5,5) facing East; emplacement at (6,5); enemy further East at (7,5) — in arc.
        let situation = SituationBuilder::new()
            .with_gangers([
                player_at(ground(5, 5), Direction::East),
                enemy_at(ground(7, 5), Direction::West),
            ])
            .build_with_gangs();
        drive_setup(&mut app, seed, situation);
        let emplacement = spawn_emplacement(&mut app, ground(6, 5));
        // The player is the gang-0 ganger at (5,5).
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &Faction, &gdtf_battle_sim::ganger::Position)>();
        let actor = q
            .iter(world)
            .find(|(_, f, p)| ***f == PLAYER && ***p == ground(5, 5))
            .map(|(e, ..)| e);
        let Some(actor) = actor else {
            unreachable!("the player ganger spawned at (5,5)");
        };

        // ENTER — man the emplacement (spawns + wields the mount).
        app.world_mut()
            .write_message(EnterEmplacementRequested::new(actor, emplacement));
        step(&mut app, 3);
        assert!(
            wields_mount(&mut app, actor),
            "precondition: the occupant wields the mount before firing",
        );

        // FIRE at the enemy cell — the ranged read PREFERS the mount while occupied.
        let mode = FireModeSpec::new(
            ModeKind::Single,
            ModeConeMult::new(1.0),
            ModeTuPercent::new(0.3),
            ModeShots::new(1),
        );
        app.world_mut().write_message(FireRequested::new(
            actor,
            mode,
            Cell::new(7, 5),
            Level::new(0),
        ));
        step(&mut app, 3);
        first_shot_damage_type(&app)
    };

    let a = run(0x5543_0E0E);
    let b = run(0x5543_0F0F);
    assert_eq!(
        a,
        Some(MOUNT_DAMAGE_TYPE),
        "FIRE: the occupied ganger's shot carries the MOUNTED gun's DamageType (the mount was \
         resolved, not the ganger's own {OWN_DAMAGE_TYPE:?} gun)",
    );
    assert_eq!(
        a, b,
        "FIRE: the mounted-gun shot is seed-independent in WHICH weapon fires (both seeds resolve \
         the mount) — determinism of the resolution",
    );
}
