//! A melee strike charges exactly what `melee_tu_cost` quotes for the weapon.

use bevy::{app::App, ecs::relationship::Relationship, prelude::Entity};
use gdtf_battle_sim::{
    acts::{MeleeAttacker, MeleeReach, MeleeRequested, can_melee, melee_tu_cost},
    ganger::{Direction, Faction, LifeState, Position},
    test_support::SituationBuilder,
    weapon::{FightMode, FightModeKind, FightModeSpec, Strikes, TuCost, WieldedBy},
};

use super::harness::*;

fn wielded_fight_mode(app: &mut App, ganger: Entity) -> Option<FightMode> {
    let world = app.world_mut();
    let mut held = world.query::<(&WieldedBy, &FightMode)>();
    held.iter(world)
        .find(|(wielded_by, _)| wielded_by.get() == ganger)
        .map(|(_, mode)| mode.clone())
}

#[test]
fn a_melee_strike_charges_exactly_the_melee_quote() {
    let mut app = battle_app();
    with_melee_log(&mut app);

    let situation = SituationBuilder::new()
        .with_gangers([
            strong_attacker(ground(5, 5), PLAYER, Direction::East),
            defenceless_target(ground(6, 5), ENEMY),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let (Some(attacker), Some(target)) = (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
    else {
        unreachable!("setup spawns one player and one enemy");
    };
    let Some(fight_mode) = wielded_fight_mode(&mut app, attacker) else {
        unreachable!("the attacker wields a melee weapon with a fight mode");
    };
    let quoted = melee_tu_cost(&fight_mode);
    assert!(*quoted > 0, "the test melee weapon has a real TU cost");
    let Some(tu_before) = tu_of(&app, attacker) else {
        unreachable!("the attacker carries Tu");
    };

    app.world_mut()
        .write_message(MeleeRequested::new(attacker, target));
    step(&mut app, 3);

    assert_eq!(
        tu_of(&app, attacker).map(|after| tu_before - after),
        Some(*quoted),
        "the strike charged exactly what melee_tu_cost quoted for the wielded weapon",
    );
}

#[test]
fn melee_tu_cost_reads_the_primary_modes_cost() {
    let primary = FightModeSpec::new(FightModeKind::Swing, TuCost::new(23), Strikes::new(1));
    let secondary = FightModeSpec::new(FightModeKind::Thrust, TuCost::new(7), Strikes::new(2));
    let modes = FightMode::new(vec![primary, secondary]);

    assert_ne!(
        *primary.tu_cost, *secondary.tu_cost,
        "the two modes must cost different TU or the quote cannot be told apart",
    );
    assert_eq!(
        u16::from(*melee_tu_cost(&modes)),
        *primary.tu_cost,
        "melee_tu_cost must quote the primary mode's cost, not another mode's",
    );
}

#[test]
fn can_melee_matches_the_reach_gates_the_resolvers_enforce() {
    let attacker = MeleeAttacker::new(Position::new(ground(5, 5)), Faction::new(PLAYER));
    let hostile = MeleeReach::ganger(
        Position::new(ground(6, 5)),
        Faction::new(ENEMY),
        LifeState::Alive,
    );
    assert!(
        *can_melee(attacker, hostile),
        "an adjacent, hostile, active target is a legal strike",
    );
    assert!(
        !*can_melee(
            attacker,
            MeleeReach::ganger(
                Position::new(ground(8, 5)),
                Faction::new(ENEMY),
                LifeState::Alive,
            ),
        ),
        "a non-adjacent target is not a legal strike",
    );
    assert!(
        !*can_melee(
            attacker,
            MeleeReach::ganger(
                Position::new(ground(6, 5)),
                Faction::new(PLAYER),
                LifeState::Alive,
            ),
        ),
        "a same-faction ally is not a legal strike",
    );
    assert!(
        !*can_melee(
            attacker,
            MeleeReach::ganger(
                Position::new(ground(6, 5)),
                Faction::new(ENEMY),
                LifeState::Downed,
            ),
        ),
        "a downed target is not a legal strike",
    );
    assert!(
        *can_melee(attacker, MeleeReach::structure(ground(6, 6))),
        "an adjacent structure cell is a legal smash",
    );
    assert!(
        !*can_melee(attacker, MeleeReach::structure(ground(9, 9))),
        "a distant structure cell is not a legal smash",
    );
}
