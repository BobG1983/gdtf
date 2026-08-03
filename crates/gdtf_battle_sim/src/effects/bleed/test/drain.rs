use super::support::{BleedingOut, LifeState, Wounds, bleed_app, bleed_rate, life_of, wounds_of};

#[test]
fn one_tick_drains_exactly_bleed_rate_from_a_downed_ganger() {
    let rate = bleed_rate();
    let start = rate.saturating_add(10);

    let mut app = bleed_app();
    let ganger = app
        .world_mut()
        .spawn((Wounds::new(start), LifeState::Downed, BleedingOut))
        .id();

    app.update();

    assert_eq!(
        wounds_of(&app, ganger),
        start - rate,
        "one tick must drain exactly the tuning bleed_rate from a Downed ganger",
    );
    assert_eq!(
        life_of(&app, ganger),
        LifeState::Downed,
        "a non-lethal tick must leave the ganger Downed",
    );
}

#[test]
fn ticks_stack_and_deplete_to_dead() {
    let rate = bleed_rate();
    let rounds = 5u8;
    let start = rate.saturating_mul(rounds);

    let mut app = bleed_app();
    let ganger = app
        .world_mut()
        .spawn((Wounds::new(start), LifeState::Downed, BleedingOut))
        .id();

    for n in 1..=3u8 {
        app.update();
        assert_eq!(
            wounds_of(&app, ganger),
            start - rate * n,
            "after {n} ticks the cumulative drop must be {n}×bleed_rate (a stack)",
        );
        assert_eq!(
            life_of(&app, ganger),
            LifeState::Downed,
            "while Wounds remain the ganger stays Downed",
        );
    }

    for _ in 0..rounds {
        app.update();
    }

    assert_eq!(
        wounds_of(&app, ganger),
        0,
        "bleeding to the floor must leave Wounds at 0 (saturating, no underflow)",
    );
    assert_eq!(
        life_of(&app, ganger),
        LifeState::Dead,
        "Wounds depleted to 0 by the clock must transition the ganger to Dead",
    );
}
