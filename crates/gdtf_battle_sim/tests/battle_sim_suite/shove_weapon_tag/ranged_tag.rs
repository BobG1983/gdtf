use gdtf_battle_sim::{
    acts::shove_tu_cost,
    ganger::{Aiming, Direction, TuMax},
    magazine::mode_tu_cost,
    prelude::Cell,
    test_support::SituationBuilder,
    weapon::{FireModeSpec, ModeConeMult, ModeKind, ModeShots, ModeTuPercent},
};

use super::harness::*;

#[test]
fn shove_tagged_ranged_connect_knocks_target_back() {
    let mut app = battle_app(false, true);
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

const fn single_shot_mode() -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.2),
        ModeShots::new(1),
    )
}

#[test]
fn shove_tagged_ranged_knocks_back_from_a_pool_below_the_shove_quote() {
    let mut app = battle_app(false, true);
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

    let tuning = tuning_of(&app);
    let quoted = shove_tu_cost(&tuning);
    assert!(
        *quoted > 0,
        "the shove leaf must cost something, or a below-quote pool proves nothing"
    );
    let (Some(tu_max), Some(aiming)) = (
        app.world().get::<TuMax>(shooter).copied(),
        app.world().get::<Aiming>(shooter).copied(),
    ) else {
        unreachable!("a spawned shooter carries a TU pool and an aiming state");
    };
    let shot = mode_tu_cost(&single_shot_mode(), &tu_max, &aiming, &tuning);
    let pool_before = *shot + *quoted - 1;
    set_tu(&mut app, shooter, pool_before);

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
        "a `shove`-tagged gun knocks the target back even when the pool left after the shot \
         cannot cover the shove quote"
    );
    assert!(
        tu_of(&app, shooter) < *quoted,
        "the pool the shove gate saw was below the shove quote"
    );
    assert_eq!(
        tu_of(&app, shooter),
        pool_before - *shot,
        "the weapon-tag shove charged nothing on top of the shot"
    );
}

#[test]
fn non_shove_ranged_connect_does_not_knock_back() {
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

#[test]
fn shove_tagged_ranged_miss_does_not_knock_back() {
    let mut app = battle_app(false, true);
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
