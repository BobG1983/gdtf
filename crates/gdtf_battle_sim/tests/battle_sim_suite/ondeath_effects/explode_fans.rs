use gdtf_battle_sim::{
    acts::{FireRequested, MeleeRequested},
    ganger::Direction,
    metric::{Cell, Level},
    prelude::LifeState,
    test_support::{SituationBuilder, single_mode},
    weapon::{
        BlastRadius, FireModeSpec, HitType, ModeConeMult, ModeKind, ModeShots, ModeTuPercent,
    },
};

use super::{fixtures::*, harness::*};

#[test]
fn a_killed_ganger_with_explode_on_death_damages_an_adjacent_ganger() {
    let (mut app, seed) = battle_app(0x5547_0A0A, false);

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

    app.world_mut().write_message(FireRequested::new(
        shooter_e,
        single_mode(0.2, 1),
        Cell::new(8, 5),
        Level::new(0),
    ));
    step(&mut app, 4);

    assert!(
        took_damage(neighbour0, vitals(&app, neighbour)),
        "the adjacent ganger took damage from the killed ganger's on-death Explode blast \
         (before {neighbour0:?}, after {:?})",
        vitals(&app, neighbour),
    );
}

#[test]
fn a_ganger_killed_in_melee_fans_its_on_death_explode() {
    let (mut app, seed) = battle_app(0x5547_0C0C, false);
    app.world_mut().insert_resource(lethal_melee_registry());

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

    app.world_mut()
        .write_message(MeleeRequested::new(attacker, victim));
    step(&mut app, 4);

    assert_eq!(
        life_of(&app, victim),
        LifeState::Dead,
        "the lethal melee strike killed the victim"
    );
    assert!(
        took_damage(neighbour0, vitals(&app, neighbour)),
        "the adjacent ganger took damage from the melee-killed ganger's on-death Explode blast \
         (before {neighbour0:?}, after {:?})",
        vitals(&app, neighbour),
    );
}

#[test]
fn a_weapon_authoring_two_effects_fires_both_at_the_wielders_cell() {
    let (mut app, seed) = battle_app(0x5547_2A2A, false);
    app.insert_resource(ordered_weapon_registry(vec![
        explode_effect(),
        leave_field_effect(burning()),
    ]));
    insert_field_defs(&mut app);

    let situation = SituationBuilder::new()
        .with_gangers([
            shooter(ground(5, 5), Direction::East),
            frail_target(ground(8, 5)),
            bystander(ground(8, 4)),
        ])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let (Some(shooter_e), Some(victim), Some(neighbour)) = (
        ganger_at(&mut app, ground(5, 5)),
        ganger_at(&mut app, ground(8, 5)),
        ganger_at(&mut app, ground(8, 4)),
    ) else {
        unreachable!("setup spawns the shooter + victim + neighbour at distinct cells");
    };
    let neighbour0 = vitals(&app, neighbour);

    app.world_mut().write_message(FireRequested::new(
        shooter_e,
        single_mode(0.2, 1),
        Cell::new(8, 5),
        Level::new(0),
    ));
    step(&mut app, 4);

    assert_eq!(
        life_of(&app, victim),
        LifeState::Dead,
        "the shot killed the wielder, so its weapon's on-death list fans"
    );
    assert!(
        took_damage(neighbour0, vitals(&app, neighbour)),
        "the list's first effect fanned: the adjacent ganger took the Explode's blast (before \
         {neighbour0:?}, after {:?})",
        vitals(&app, neighbour),
    );
    assert_eq!(
        field_def_at(&app, ground(8, 5)),
        Some(burning_def()),
        "the list's second effect fanned: the burning def stands at the wielder's cell",
    );
}

#[test]
fn a_weapons_later_leave_field_is_the_one_the_cell_keeps() {
    let (mut app, seed) = battle_app(0x5547_2B2B, false);
    app.insert_resource(ordered_weapon_registry(vec![
        leave_field_effect(burning()),
        leave_field_effect(toxic()),
    ]));
    insert_field_defs(&mut app);

    let situation = SituationBuilder::new()
        .with_gangers([
            shooter(ground(5, 5), Direction::East),
            frail_target(ground(8, 5)),
        ])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let (Some(shooter_e), Some(victim)) = (
        ganger_at(&mut app, ground(5, 5)),
        ganger_at(&mut app, ground(8, 5)),
    ) else {
        unreachable!("setup spawns the shooter and the victim at distinct cells");
    };

    app.world_mut().write_message(FireRequested::new(
        shooter_e,
        single_mode(0.2, 1),
        Cell::new(8, 5),
        Level::new(0),
    ));
    step(&mut app, 4);

    assert_eq!(
        life_of(&app, victim),
        LifeState::Dead,
        "the shot killed the wielder, so its weapon's on-death list fans"
    );
    assert_eq!(
        field_def_at(&app, ground(8, 5)).map(|def| def.damage_type),
        Some(toxic_def().damage_type),
        "both LeaveField effects fanned at the one cell, so the cell keeps the LAST authored \
         def; it holds {:?}",
        field_def_at(&app, ground(8, 5)),
    );
}

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
    assert!(
        took_damage(neighbour0, vitals(&app, neighbour)),
        "the neighbour took damage from the splash-killed ganger's on-death Explode blast \
         (before {neighbour0:?}, after {:?})",
        vitals(&app, neighbour),
    );
}
