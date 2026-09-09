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

#[test]
fn the_three_per_round_drains_record_in_the_recorders_fixed_order() {
    let mut app = battle_app(forced_reaction_tuning(1));
    let at = ground(2, 2);
    drive_setup(&mut app, two_ganger_situation(at, ground(6, 2)));
    let Some(ganger) = ganger_at(&mut app, at) else {
        unreachable!("the fixture spawns a ganger at the drained cell");
    };

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
