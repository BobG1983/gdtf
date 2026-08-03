use super::support::{
    Hp, LifeState, Position, dot, dot_of, ground, hp_of, life_of, tick_app, tick_count_for,
};

// === (c) tick_dot decrements HP each round with NO armor/injury/RNG, removes after N turns. ===

#[test]
fn tick_dot_drains_hp_each_round_and_removes_after_the_profile_turn_count() {
    let per_turn = 5u16;
    let turns = 3u8;
    let start_hp = 100u16;

    let mut app = tick_app();
    let ganger = app
        .world_mut()
        .spawn((
            Hp::new(start_hp),
            LifeState::Alive,
            dot(per_turn, turns),
            Position::new(ground(4, 4)),
        ))
        .id();

    for n in 1..=turns {
        app.update();
        assert_eq!(
            hp_of(&app, ganger),
            start_hp - per_turn * u16::from(n),
            "after {n} ticks the cumulative HP drop must be {n}×per_turn (a flat direct drain)",
        );
        assert_eq!(
            life_of(&app, ganger),
            LifeState::Alive,
            "a non-lethal DOT tick leaves the ganger Alive",
        );
        assert_eq!(
            dot_of(&app, ganger).map(|d| d.remaining_turns.get()),
            (n < turns).then_some(turns - n),
            "after {n} of {turns} ticks the DOT is decremented in place, then REMOVED on \
             the expiring tick — never stored at zero",
        );
    }

    assert!(
        dot_of(&app, ganger).is_none(),
        "the DOT is removed once its profile turn count runs out",
    );
    let hp_after_expiry = hp_of(&app, ganger);
    app.update();
    assert_eq!(
        hp_of(&app, ganger),
        hp_after_expiry,
        "with the DOT removed a further round drains no HP",
    );
    assert_eq!(
        tick_count_for(&app, ganger),
        usize::from(turns),
        "exactly one DotTicked per turn of the profile (none after removal)",
    );
}


#[test]
fn the_expiring_tick_removes_the_dot_not_an_inert_component() {
    let mut app = tick_app();
    let ganger = app
        .world_mut()
        .spawn((
            Hp::new(50),
            LifeState::Alive,
            dot(5, 1),
            Position::new(ground(1, 1)),
        ))
        .id();

    app.update();

    assert_eq!(
        hp_of(&app, ganger),
        45,
        "the one-turn DOT drains exactly once before expiring",
    );
    assert_eq!(
        tick_count_for(&app, ganger),
        1,
        "exactly one DotTicked for the one-turn DOT",
    );
    assert!(
        dot_of(&app, ganger).is_none(),
        "the expiring tick must REMOVE the Dot component (decrement-or-REMOVE, GTW-643)",
    );
}


#[test]
fn a_dot_tick_that_empties_hp_flips_the_ganger_to_dead() {
    let mut app = tick_app();
    let ganger = app
        .world_mut()
        .spawn((
            Hp::new(8),
            LifeState::Alive,
            dot(10, 3),
            Position::new(ground(2, 2)),
        ))
        .id();

    app.update();

    assert_eq!(
        hp_of(&app, ganger),
        0,
        "the DOT tick floors HP at 0 (saturating, no underflow)",
    );
    assert_eq!(
        life_of(&app, ganger),
        LifeState::Dead,
        "the GTW-544 locked design: a DOT tick that empties HP KILLS (Dead, not Downed)",
    );
    assert!(
        dot_of(&app, ganger).is_none(),
        "the DOT is removed on the killing tick",
    );
}


#[test]
fn an_already_dead_hosts_dot_is_skipped_untouched() {
    let start_hp = 30u16;
    let turns = 2u8;

    let mut app = tick_app();
    let corpse = app
        .world_mut()
        .spawn((
            Hp::new(start_hp),
            LifeState::Dead,
            dot(5, turns),
            Position::new(ground(3, 3)),
        ))
        .id();

    app.update();

    assert_eq!(
        hp_of(&app, corpse),
        start_hp,
        "a corpse's DOT drains nothing (the once-only property)",
    );
    assert_eq!(
        tick_count_for(&app, corpse),
        0,
        "a corpse's DOT emits no DotTicked",
    );
    assert_eq!(
        dot_of(&app, corpse).map(|d| d.remaining_turns.get()),
        Some(turns),
        "the corpse's Dot remains, inert and undecremented (the pinned Dead-path contract)",
    );
}
