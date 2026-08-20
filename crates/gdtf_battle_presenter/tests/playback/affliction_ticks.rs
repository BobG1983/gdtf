use std::time::Duration;

use gdtf_battle_sim::{
    act_log::{ActDeed, ActProvenance},
    effects::{
        dot::DotTicked,
        fields::{FieldDamage, FieldTicked},
    },
    ganger::Direction,
    weapon::DotDamage,
};

use super::harness::*;

#[test]
fn a_recorded_dot_drain_is_played_with_its_cell_and_amount() {
    let mut app = playback_app();
    let at = ground(3, 5);
    let ganger = spawn_ganger(&mut app, at, Direction::North);
    seed(&mut app);

    append(
        &mut app,
        ganger,
        ActProvenance::Clock,
        ActDeed::DotTicked {
            at,
            amount: DotDamage::new(4),
        },
    );
    step(&mut app, Duration::ZERO);

    let ticks = played::<DotTicked>(&mut app);
    assert_eq!(
        ticks.len(),
        1,
        "showing a DotTicked deed must emit exactly one Played<DotTicked>",
    );
    let Some(tick) = ticks.first() else { return };
    assert_eq!(
        tick.ganger, ganger,
        "the played drain names the drained ganger"
    );
    assert_eq!(tick.at, at, "the played drain carries the recorded cell");
    assert_eq!(
        tick.amount,
        DotDamage::new(4),
        "the played drain carries the recorded amount",
    );
}

#[test]
fn a_recorded_field_drain_is_played_with_its_cell_and_amount() {
    let mut app = playback_app();
    let at = ground(8, 2);
    let occupant = spawn_ganger(&mut app, at, Direction::East);
    seed(&mut app);

    append(
        &mut app,
        occupant,
        ActProvenance::Clock,
        ActDeed::FieldTicked {
            at,
            amount: FieldDamage::new(3),
        },
    );
    step(&mut app, Duration::ZERO);

    let ticks = played::<FieldTicked>(&mut app);
    assert_eq!(
        ticks.len(),
        1,
        "showing a FieldTicked deed must emit exactly one Played<FieldTicked>",
    );
    let Some(tick) = ticks.first() else { return };
    assert_eq!(
        tick.occupant, occupant,
        "the played drain names the drained occupant",
    );
    assert_eq!(tick.at, at, "the played drain carries the recorded cell");
    assert_eq!(
        tick.amount,
        FieldDamage::new(3),
        "the played drain carries the recorded amount",
    );
}

#[test]
fn showing_a_drain_holds_the_cursor_for_the_minor_beat() {
    for (first, second) in [
        (
            ActDeed::DotTicked {
                at:     ground(4, 4),
                amount: DotDamage::new(1),
            },
            ActDeed::FieldTicked {
                at:     ground(4, 4),
                amount: FieldDamage::new(1),
            },
        ),
        (
            ActDeed::FieldTicked {
                at:     ground(6, 6),
                amount: FieldDamage::new(2),
            },
            ActDeed::DotTicked {
                at:     ground(6, 6),
                amount: DotDamage::new(2),
            },
        ),
    ] {
        let mut app = playback_app();
        let ganger = spawn_ganger(&mut app, ground(4, 4), Direction::North);
        seed(&mut app);
        append(&mut app, ganger, ActProvenance::Clock, first.clone());
        append(&mut app, ganger, ActProvenance::Clock, second);

        step(&mut app, Duration::ZERO);
        assert!(
            holding(&app),
            "showing {first:?} must start a hold, not release instantly",
        );

        let beat = *tuning(&app).minor_seconds;
        step(&mut app, Duration::from_secs_f32(beat * 0.5));
        assert_eq!(
            *shown(&app),
            1,
            "half the minor beat must not serve {first:?}'s hold",
        );

        step(&mut app, Duration::from_secs_f32(beat));
        assert_eq!(
            *shown(&app),
            2,
            "once {first:?}'s minor beat is served the next drain is shown",
        );
    }
}

#[test]
fn a_drain_behind_a_held_entry_is_not_played_yet() {
    let mut app = playback_app();
    let at = ground(1, 1);
    let ganger = spawn_ganger(&mut app, at, Direction::North);
    let other = spawn_ganger(&mut app, ground(2, 2), Direction::East);
    seed(&mut app);

    append_reaction_fire(&mut app, other, ganger);
    append(
        &mut app,
        ganger,
        ActProvenance::Clock,
        ActDeed::DotTicked {
            at,
            amount: DotDamage::new(2),
        },
    );

    step(&mut app, Duration::ZERO);
    assert!(
        holding(&app),
        "the fire declaration's beat must be in progress",
    );
    assert!(
        played::<DotTicked>(&mut app).is_empty(),
        "a drain the cursor has not reached must not be played",
    );

    let beat = *tuning(&app).reaction_beat_seconds;
    step(&mut app, Duration::from_secs_f32(beat));
    assert_eq!(
        played::<DotTicked>(&mut app).len(),
        1,
        "once its beat is served the drain is played",
    );
}
