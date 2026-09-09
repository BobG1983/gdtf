use bevy::{
    app::App,
    prelude::{Entity, Resource},
};
use gdtf_battle_sim::{
    acts::{EnterEmplacementRequested, FireRequested},
    ganger::{Aiming, Direction, Facing, LifeState, Toughness, TuMax},
    magazine::mode_tu_cost,
    metric::{Cell, CellLevel, Level},
    prelude::{Faction, Stance, StanceKind},
    shot_fired::ShotFired,
    situation::GangerSpawn,
    test_support::{GangerSpawnBuilder, SituationBuilder, emplacement_at},
    tuning::{CombatTuning, ViewRange},
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

/// How many shots the log holds right now, which a caller uses as a start index.
fn shots_logged(app: &App) -> usize {
    app.world()
        .get_resource::<ShotLog>()
        .map_or(0, |log| log.shots.len())
}

/// The index and damage type of the first shot `shooter` fired at or after `from`.
fn shot_at_or_after(app: &App, shooter: Entity, from: usize) -> Option<(usize, DamageType)> {
    let log = app.world().get_resource::<ShotLog>()?;
    log.shots
        .iter()
        .enumerate()
        .skip(from)
        .find(|(_, shot)| shot.shooter == shooter)
        .map(|(index, shot)| (index, shot.damage))
}

/// What firing `mode` charges this actor, or nothing when the world holds no price for it.
fn cost_of(app: &App, actor: Entity, mode: &FireModeSpec) -> Option<u8> {
    let world = app.world();
    let tu_max = world.get::<TuMax>(actor)?;
    let aiming = world.get::<Aiming>(actor)?;
    let tuning = world.get_resource::<CombatTuning>()?;
    Some(*mode_tu_cost(mode, tu_max, aiming, tuning))
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
        // Force the enter-provoked interrupt rather than leaving it to a roll.
        app.insert_resource(CombatTuning {
            view_range: ViewRange::new(TEST_VIEW_RANGE),
            reaction: forced_reaction_tuning(1),
            ..Default::default()
        });
        with_shot_log(&mut app);
        let situation = SituationBuilder::new()
            .with_gangers([
                player_at(ground(5, 5), Direction::East),
                enemy_at(ground(7, 5), Direction::West),
            ])
            .with_scatter(emplacement_at(ground(6, 5)))
            .build_with_gangs();
        drive_setup(&mut app, seed, situation);
        let emplacement = seated_emplacement(&mut app, ground(6, 5));
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
        assert!(
            matches!(app.world().get::<LifeState>(actor), Some(LifeState::Alive)),
            "precondition: the enter exchange left the actor alive to take the shot under test",
        );
        let Some(cost) = cost_of(&app, actor, &mode) else {
            unreachable!("the actor carries TuMax + Aiming and the app carries CombatTuning");
        };
        assert!(
            tu_of(&app, actor).is_some_and(|tu| tu >= cost),
            "precondition: after the enter exchange the actor's pool still covers the {cost} TU \
             the mode under test charges",
        );

        let from = shots_logged(&app);
        app.world_mut().write_message(FireRequested::new(
            actor,
            mode,
            Cell::new(7, 5),
            Level::new(0),
        ));
        step(&mut app, 3);
        let (index, damage) = shot_at_or_after(&app, actor, from)?;
        assert!(
            index >= from,
            "FIRE: the shot read must be the DELIBERATE one, logged at or after index {from}; \
             a reader ignoring the start index picks up the actor's own reaction-pass mount \
             shot at {index} instead, which fires the same weapon and so looks identical",
        );
        Some(damage)
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
