use bevy::{
    app::App,
    prelude::{Entity, MessageReader, ResMut, Resource},
};
use gdtf_battle_sim::{
    effects::attachments::AttachmentEffect, ganger::Direction, prelude::Cell,
    suppression::SuppressionApplied, test_support::SituationBuilder,
};

use super::harness::*;

#[derive(Resource, Default)]
struct AppliedLog {
    count: usize,
}

fn record_applied(mut msgs: MessageReader<SuppressionApplied>, mut log: ResMut<AppliedLog>) {
    for _msg in msgs.read() {
        log.count += 1;
    }
}

fn suppression_probe_app(effects: Vec<AttachmentEffect>) -> (App, Entity) {
    let mut app = battle_app(effects);
    app.init_resource::<AppliedLog>();
    app.add_systems(bevy::app::Update, record_applied);
    let situation = SituationBuilder::new()
        .with_gangers([
            player_at(ground(5, 5), Direction::East),
            enemy_at(ground(6, 5)),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);
    let Some(shooter) = ganger_of(&mut app, PLAYER) else {
        unreachable!("setup spawns one player");
    };
    (app, shooter)
}

#[test]
fn silence_effect_still_gates_suppression() {
    let (mut loud, loud_shooter) = suppression_probe_app(Vec::new());
    loud.world_mut()
        .write_message(gdtf_battle_sim::acts::FireRequested::new(
            loud_shooter,
            single_shot_mode(),
            Cell::new(6, 5),
            level0(),
        ));
    for _ in 0..4 {
        loud.update();
    }
    let loud_count = loud.world().resource::<AppliedLog>().count;
    assert!(
        loud_count > 0,
        "an UN-silenced shot suppresses the adjacent enemy (control: {loud_count} signals)",
    );

    let (mut quiet, quiet_shooter) = suppression_probe_app(vec![AttachmentEffect::Silence]);
    quiet
        .world_mut()
        .write_message(gdtf_battle_sim::acts::FireRequested::new(
            quiet_shooter,
            single_shot_mode(),
            Cell::new(6, 5),
            level0(),
        ));
    for _ in 0..4 {
        quiet.update();
    }
    assert_eq!(
        quiet.world().resource::<AppliedLog>().count,
        0,
        "a SILENCED shot produces NO SuppressionApplied (the PRESERVED producer gate reads the \
         shooter's Silenced weapon)",
    );
}
