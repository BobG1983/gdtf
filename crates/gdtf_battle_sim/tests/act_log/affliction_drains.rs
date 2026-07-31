//! GTW-889 — the per-round DOT and FIELD drains are RECORDED, on the real
//! `BattleSimPlugin` `Record` path.
//!
//! They were the two facts the presenter pops floating text for that the log said nothing
//! about. A fact with no entry has no place in the ordered account, so its pop could only
//! fire when the SIM produced it — ahead of the shots and turn beats a consumer was still
//! working through. These tests pin the two entries and their order against the bleed drain
//! they sit beside.

use gdtf_battle_sim::{
    act_log::ActDeed,
    effects::{
        bleed::Bleeding,
        dot::DotTicked,
        fields::{FieldDamage, FieldTicked},
    },
    ganger::Direction,
    test_support::SituationBuilder,
    weapon::DotDamage,
};

use super::harness::*;

/// One player watcher and one enemy mover — the minimal live battle these drains ride in.
fn two_ganger_situation(
    player_at: gdtf_battle_sim::metric::CellLevel,
    enemy_at: gdtf_battle_sim::metric::CellLevel,
) -> (
    gdtf_battle_sim::situation::Situation,
    gdtf_battle_sim::ganger::GangRegistry,
) {
    SituationBuilder::new()
        .with_gangers([
            watcher(player_at, PLAYER, Direction::East),
            tough_mover(enemy_at, ENEMY, Direction::West),
        ])
        .build_with_gangs()
}

/// A DOT drain the sim emits is recorded as its own entry, attributed to the drained
/// ganger, as a CLOCK beat (the ganger is the subject of the tick, not its author) —
/// carrying the emitting message's OWN anchor cell and drained amount.
///
/// The message's `at` is deliberately a cell the ganger is NOT standing in: the deed's
/// anchor must come from the message the recorder is reading, not from the live position
/// query it happens to sit beside. With the two equal, a recorder that re-derived the
/// anchor from live state would pass.
#[test]
fn a_dot_drain_is_recorded_as_a_clock_beat_on_the_drained_ganger() {
    let mut app = battle_app(forced_reaction_tuning(1));
    let mover_at = ground(2, 2);
    let drained_at = ground(9, 9);
    drive_setup(&mut app, two_ganger_situation(mover_at, ground(6, 2)));
    let Some(ganger) = ganger_at(&mut app, mover_at) else {
        unreachable!("the fixture spawns a ganger at the mover cell");
    };

    app.world_mut()
        .write_message(DotTicked::new(ganger, drained_at, DotDamage::new(4)));
    app.update();

    let drains = logged_of(&app, "DotTicked");
    assert_eq!(
        drains.len(),
        1,
        "one emitted DOT drain records exactly one DotTicked entry, log was {:?}",
        logged(&app),
    );
    let Some(drain) = drains.first() else { return };
    assert_eq!(
        drain.actor, ganger,
        "the recorded drain names the drained ganger",
    );
    assert_eq!(
        drain.provenance,
        gdtf_battle_sim::act_log::ActProvenance::Clock,
        "an affliction drain is a clock beat, not a commanded or AI act",
    );
    assert_eq!(
        deeds_of(&app, "DotTicked"),
        vec![ActDeed::DotTicked {
            at:     drained_at,
            amount: DotDamage::new(4),
        }],
        "the recorded deed carries the message's OWN anchor cell and drained HP — the \
         after-values a consumer applies rather than re-derives",
    );
}

/// A FIELD drain the sim emits is recorded as its own entry, attributed to the drained
/// occupant, carrying the emitting message's OWN field cell and drained amount.
///
/// As above, the message's `at` is a cell the occupant is NOT standing in, so the
/// assertion pins the message → deed mapping rather than a live-position read.
#[test]
fn a_field_drain_is_recorded_as_a_clock_beat_on_the_occupant() {
    let mut app = battle_app(forced_reaction_tuning(1));
    let occupant_at = ground(2, 2);
    let field_at = ground(9, 9);
    drive_setup(&mut app, two_ganger_situation(occupant_at, ground(6, 2)));
    let Some(occupant) = ganger_at(&mut app, occupant_at) else {
        unreachable!("the fixture spawns a ganger at the occupant cell");
    };

    app.world_mut()
        .write_message(FieldTicked::new(occupant, field_at, FieldDamage::new(3)));
    app.update();

    let drains = logged_of(&app, "FieldTicked");
    assert_eq!(
        drains.len(),
        1,
        "one emitted field drain records exactly one FieldTicked entry, log was {:?}",
        logged(&app),
    );
    let Some(drain) = drains.first() else { return };
    assert_eq!(
        drain.actor, occupant,
        "the recorded drain names the drained occupant",
    );
    assert_eq!(
        deeds_of(&app, "FieldTicked"),
        vec![ActDeed::FieldTicked {
            at:     field_at,
            amount: FieldDamage::new(3),
        }],
        "the recorded deed carries the message's OWN field cell and drained HP — the \
         after-values a consumer applies rather than re-derives",
    );
}

/// The three per-round drains recorded in ONE tick keep the recorder's fixed source order —
/// bleed, then DOT, then field — so the log is a property of the recorder's source code
/// rather than of the scheduler.
#[test]
fn the_three_per_round_drains_record_in_the_recorders_fixed_order() {
    let mut app = battle_app(forced_reaction_tuning(1));
    let at = ground(2, 2);
    drive_setup(&mut app, two_ganger_situation(at, ground(6, 2)));
    let Some(ganger) = ganger_at(&mut app, at) else {
        unreachable!("the fixture spawns a ganger at the drained cell");
    };

    // Written in the OPPOSITE order to the one they must be recorded in, so a recorder that
    // simply followed the write order would fail this.
    app.world_mut()
        .write_message(FieldTicked::new(ganger, at, FieldDamage::new(3)));
    app.world_mut()
        .write_message(DotTicked::new(ganger, at, DotDamage::new(4)));
    app.world_mut().write_message(Bleeding::new(ganger));
    app.update();

    let order: Vec<&'static str> = logged(&app)
        .into_iter()
        .map(|fact| fact.deed)
        .filter(|deed| matches!(*deed, "Bled" | "DotTicked" | "FieldTicked"))
        .collect();
    assert_eq!(
        order,
        vec!["Bled", "DotTicked", "FieldTicked"],
        "the drains record in the recorder's fixed source order, log was {:?}",
        logged(&app),
    );
}
