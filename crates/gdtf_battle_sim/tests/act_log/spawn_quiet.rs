use gdtf_battle_sim::{
    battle::{BattleLost, BattleWon},
    ganger::Direction,
    test_support::SituationBuilder,
};

use super::harness::*;

#[test]
fn spawning_a_roster_appends_no_entries() {
    let mut app = battle_app(forced_reaction_tuning(1));

    let situation = SituationBuilder::new()
        .with_gangers([
            watcher(ground(5, 4), PLAYER, Direction::East),
            watcher(ground(5, 6), PLAYER, Direction::East),
            tough_mover(ground(9, 5), ENEMY, Direction::West),
            tough_mover(ground(9, 7), ENEMY, Direction::West),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let entries = logged(&app);
    assert!(
        entries.is_empty(),
        "spawning a roster must record NOTHING — every ganger's spawn-time facing / stance \
         / aim / position / life state is a FIRST observation, which seeds the transition \
         map and emits no entry. Got: {entries:?}",
    );
}

#[test]
fn a_battle_with_no_presenter_runs_to_an_outcome() {
    let mut app = battle_app(forced_reaction_tuning(8));
    app.init_resource::<Outcomes>();
    app.add_systems(bevy::app::Update, record_outcomes);

    let watch = ground(5, 5);
    let situation = SituationBuilder::new()
        .with_gangers([
            watcher(watch, PLAYER, Direction::East),
            frail_mover(ground(8, 5), ENEMY, Direction::West),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let Some(mover) = ganger_at(&mut app, ground(8, 5)) else {
        unreachable!("setup spawns the mover at its fixture cell");
    };
    app.world_mut()
        .write_message(gdtf_battle_sim::acts::MoveRequested::new(
            mover,
            ground(6, 5),
        ));

    for _ in 0..64 {
        app.update();
    }

    let decided = app
        .world()
        .get_resource::<Outcomes>()
        .is_some_and(|outcomes| outcomes.decided);
    assert!(
        decided,
        "a battle with NO presenter must reach a decided outcome — nothing in the sim waits \
         on a cursor that does not exist",
    );
    assert!(
        !logged(&app).is_empty(),
        "the recorder runs headless too — the log is what a QA client and the log tests read",
    );
}

fn frail_mover(
    at: gdtf_battle_sim::metric::CellLevel,
    faction: u8,
    facing: Direction,
) -> gdtf_battle_sim::situation::GangerSpawn {
    use gdtf_battle_sim::{
        ganger::{Cool, Facing, Grit, Reflexes, Speed, Toughness},
        prelude::{Faction, Stance, StanceKind},
        test_support::GangerSpawnBuilder,
    };
    GangerSpawnBuilder::new()
        .at(at)
        .faction(Faction::new(faction))
        .facing(Facing::new(facing))
        .stance(Stance::new(StanceKind::Standing))
        .speed(Speed::new(20.0))
        .reflexes(Reflexes::new(1.0))
        .cool(Cool::new(1.0))
        .grit(Grit::new(1.0))
        .toughness(Toughness::new(1.0))
        .build()
}

#[derive(bevy::prelude::Resource, Default)]
struct Outcomes {
        decided: bool,
}

fn record_outcomes(
    mut won: bevy::prelude::MessageReader<BattleWon>,
    mut lost: bevy::prelude::MessageReader<BattleLost>,
    mut outcomes: bevy::prelude::ResMut<Outcomes>,
) {
    if won.read().next().is_some() || lost.read().next().is_some() {
        outcomes.decided = true;
    }
}
