use super::support::{
    Bleeding, BleedingOut, LifeState, Wounds, bleed_app, bleed_rate, bleeding_count_for, life_of,
    wounds_of,
};
use crate::ganger::Hp;

/// AC6 — `Bleeding` is a buffered `#[derive(Message)]` (NOT the observer `Event`
#[test]
fn bleeding_is_a_buffered_message_one_per_draining_ganger() {
    let rate = bleed_rate();
    let start = rate.saturating_add(10);

    let mut app = bleed_app();
    let bleeder = app
        .world_mut()
        .spawn((Wounds::new(start), LifeState::Downed, BleedingOut))
        .id();
    let stabilized = app
        .world_mut()
        .spawn((Wounds::new(start), LifeState::Downed))
        .id();
    let alive = app
        .world_mut()
        .spawn((Wounds::new(start), LifeState::Alive))
        .id();

    app.update();

    let captured = app
        .world()
        .get_resource::<super::support::Captured>()
        .map_or_else(Vec::new, |c| c.to_vec());

    assert_eq!(
        captured.len(),
        1,
        "exactly one Bleeding must be buffered (one per draining Downed ganger)",
    );
    assert_eq!(
        captured.first(),
        Some(&Bleeding::new(bleeder)),
        "the buffered Bleeding must carry the bleeding ganger's Entity",
    );
    assert_eq!(
        bleeding_count_for(&app, stabilized),
        0,
        "a stabilized Downed ganger emits no Bleeding",
    );
    assert_eq!(
        bleeding_count_for(&app, alive),
        0,
        "an Alive ganger emits no Bleeding",
    );
}

#[test]
fn the_lethal_tick_emits_bleeding_and_flips_to_dead() {
    let rate = bleed_rate();
    let start = rate;

    let mut app = bleed_app();
    let ganger = app
        .world_mut()
        .spawn((Wounds::new(start), LifeState::Downed, BleedingOut))
        .id();

    app.update();

    assert_eq!(
        wounds_of(&app, ganger),
        0,
        "the lethal tick must drain the pool to 0",
    );
    assert_eq!(
        life_of(&app, ganger),
        LifeState::Dead,
        "the lethal tick must flip the ganger to Dead (the terminal gate)",
    );
    assert_eq!(
        bleeding_count_for(&app, ganger),
        1,
        "the lethal tick still emits one Bleeding (drain + emit + gate on the same tick)",
    );
}

#[test]
fn a_dead_bled_out_ganger_is_skipped_next_tick() {
    let rate = bleed_rate();
    let start = rate;

    let mut app = bleed_app();
    let ganger = app
        .world_mut()
        .spawn((
            Wounds::new(start),
            LifeState::Downed,
            Hp::new(0),
            BleedingOut,
        ))
        .id();

    app.update();
    assert_eq!(life_of(&app, ganger), LifeState::Dead);
    assert_eq!(bleeding_count_for(&app, ganger), 1);

    app.update();
    assert_eq!(
        life_of(&app, ganger),
        LifeState::Dead,
        "a dead ganger stays Dead across the next tick",
    );
    assert_eq!(
        bleeding_count_for(&app, ganger),
        1,
        "a dead (bled-out) ganger emits NO second Bleeding (the once-only property)",
    );
}
