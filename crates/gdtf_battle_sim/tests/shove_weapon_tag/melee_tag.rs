use gdtf_battle_sim::{acts::MeleeRequested, ganger::Direction, test_support::SituationBuilder};

use super::harness::*;


#[test]
fn shove_tagged_melee_connect_knocks_target_back() {
    let mut app = battle_app(true, false);
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

    assert_eq!(
        pos_of(&app, target),
        Some(ground(7, 5)),
        "a `shove`-tagged melee weapon knocks the target back one cell on a connecting strike"
    );
}


#[test]
fn non_shove_melee_connect_does_not_knock_back() {
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


#[test]
fn shove_tagged_melee_miss_does_not_knock_back() {
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
