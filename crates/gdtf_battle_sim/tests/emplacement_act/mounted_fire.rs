use bevy::{
    app::App,
    prelude::{Entity, Resource},
};
use gdtf_battle_sim::{
    acts::{EnterEmplacementRequested, FireRequested},
    ganger::{Direction, Facing, Toughness},
    metric::{Cell, CellLevel, Level},
    prelude::{Faction, Stance, StanceKind},
    shot_fired::ShotFired,
    situation::GangerSpawn,
    test_support::{GangerSpawnBuilder, SituationBuilder},
    weapon::{
        DamageType, FireModeSpec, ModeConeMult, ModeKind, ModeShots, ModeTuPercent, WeaponName,
    },
};

use super::harness::*;

#[derive(Resource, Default)]
struct ShotLog {
    shots: Vec<ShotFired>,
}

fn record_shots(
    mut fired: bevy::prelude::MessageReader<ShotFired>,
    mut log: bevy::prelude::ResMut<ShotLog>,
) {
    for shot in fired.read() {
        log.shots.push(shot.clone());
    }
}

fn with_shot_log(app: &mut App) {
    app.init_resource::<ShotLog>();
    app.add_systems(bevy::app::Update, record_shots);
}

fn first_shot_damage_type(app: &App) -> Option<DamageType> {
    app.world()
        .get_resource::<ShotLog>()
        .and_then(|log| log.shots.first().map(|s| s.damage))
}

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

#[test]
fn occupied_ganger_fires_the_mounted_weapon_deterministically() {
    let run = |seed: u64| -> Option<DamageType> {
        let (mut app, seed) = battle_app(seed);
        with_shot_log(&mut app);
        let situation = SituationBuilder::new()
            .with_gangers([
                player_at(ground(5, 5), Direction::East),
                enemy_at(ground(7, 5), Direction::West),
            ])
            .build_with_gangs();
        drive_setup(&mut app, seed, situation);
        let emplacement = spawn_emplacement(&mut app, ground(6, 5));
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &Faction, &gdtf_battle_sim::ganger::Position)>();
        let actor = q
            .iter(world)
            .find(|(_, f, p)| ***f == PLAYER && ***p == ground(5, 5))
            .map(|(e, ..)| e);
        let Some(actor) = actor else {
            unreachable!("the player ganger spawned at (5,5)");
        };

        app.world_mut()
            .write_message(EnterEmplacementRequested::new(actor, emplacement));
        step(&mut app, 3);
        assert!(
            wields_mount(&mut app, actor),
            "precondition: the occupant wields the mount before firing",
        );

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
