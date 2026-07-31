//! GTW-889 — the per-round AFFLICTION DRAINS play through the cursor like everything else.
//!
//! The DOT and field per-round drains were the two facts the presenter pops floating text
//! for that had no act-log deed at all. With nothing recorded, their `"-N"` pops could only
//! ever fire at SIM time — ahead of the shots and turn beats the cursor was still playing
//! out. These tests pin the two new deeds end to end: the cursor shows them, and each
//! re-emits its sim fact as a `Played<M>` carrying the recorded cell and amount.

use std::time::Duration;

use bevy::ecs::message::Messages;
use gdtf_battle_presenter::Played;
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

/// Every `Played<M>` written since the buffer was last drained.
fn played<M: bevy::ecs::message::Message + Clone>(app: &mut bevy::app::App) -> Vec<Played<M>> {
    app.world_mut()
        .get_resource_mut::<Messages<Played<M>>>()
        .map_or_else(Vec::new, |mut messages| messages.drain().collect())
}

/// A recorded DOT drain is SHOWN by the cursor as a `Played<DotTicked>` carrying the
/// recorded cell and amount.
///
/// Pin-discriminating: with no `ActDeed::DotTicked` there is nothing for the cursor to
/// show, the pop can only be driven off the raw sim buffer, and this fails.
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

/// A recorded FIELD drain is SHOWN by the cursor as a `Played<FieldTicked>` carrying the
/// recorded cell and amount.
///
/// Pin-discriminating: exactly as above for the field family.
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

/// Showing either drain EARNS A BEAT: the cursor holds for the tuning's minor dwell, so the
/// entry behind it waits.
///
/// Value-agnostic — the expected beat is read from the resident `PlaybackTuning`, never a
/// pinned magnitude. Pin-discriminating: an `ActHold::instant()` for either deed releases
/// the second entry on the sub-beat step and FAILS here.
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

        // The first drain reaches the screen and starts its hold.
        step(&mut app, Duration::ZERO);
        assert!(
            holding(&app),
            "showing {first:?} must start a hold, not release instantly",
        );

        // A SUB-beat step must not release the entry behind it: the second drain is still
        // unshown. (This is the assertion a zero-length hold fails — `holding` alone would
        // not, since the released entry immediately starts a hold of its own.)
        let beat = *tuning(&app).minor_seconds;
        step(&mut app, Duration::from_secs_f32(beat * 0.5));
        assert_eq!(
            *shown(&app),
            1,
            "half the minor beat must not serve {first:?}'s hold",
        );

        // The rest of the beat releases it and the second drain is shown.
        step(&mut app, Duration::from_secs_f32(beat));
        assert_eq!(
            *shown(&app),
            2,
            "once {first:?}'s minor beat is served the next drain is shown",
        );
    }
}

/// A drain the cursor has NOT reached yet is not played — the whole point of recording it.
///
/// The cursor is held on an earlier entry, so the drain behind it stays unplayed. Before
/// GTW-889 the pop had no cursor to sit behind at all.
#[test]
fn a_drain_behind_a_held_entry_is_not_played_yet() {
    let mut app = playback_app();
    let at = ground(1, 1);
    let ganger = spawn_ganger(&mut app, at, Direction::North);
    let other = spawn_ganger(&mut app, ground(2, 2), Direction::East);
    seed(&mut app);

    // A fire declaration ahead of the drain: showing it starts a beat the cursor must serve
    // before it may release anything else.
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

    // Serve the beat; now the drain reaches the screen.
    let beat = *tuning(&app).reaction_beat_seconds;
    step(&mut app, Duration::from_secs_f32(beat));
    assert_eq!(
        played::<DotTicked>(&mut app).len(),
        1,
        "once its beat is served the drain is played",
    );
}
