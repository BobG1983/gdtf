use bevy::{
    app::App,
    prelude::{MessageReader, Resource},
};
use gdtf_battle_sim::{
    acts::FireRequested,
    ganger::Direction,
    metric::{Cell, CellLevel, Level},
    prelude::Position,
    suppression::SuppressionApplied,
    test_support::SituationBuilder,
};

use super::harness::*;

#[derive(Resource, Default)]
struct AppliedLog {
        cells:   Vec<CellLevel>,
            gangers: Vec<bevy::prelude::Entity>,
}

fn record_applied(
    mut msgs: MessageReader<SuppressionApplied>,
    mut log: bevy::prelude::ResMut<AppliedLog>,
) {
    for msg in msgs.read() {
        log.cells.push(msg.at);
        log.gangers.push(msg.ganger);
    }
}

fn applied_carried_ganger(app: &App, ganger: bevy::prelude::Entity) -> bool {
    app.world()
        .get_resource::<AppliedLog>()
        .is_some_and(|log| log.gangers.contains(&ganger))
}

fn with_applied_log(app: &mut App) {
    app.init_resource::<AppliedLog>();
    app.add_systems(bevy::app::Update, record_applied);
}

fn applied_count_for(app: &App, cell: CellLevel) -> usize {
    app.world()
        .get_resource::<AppliedLog>()
        .map_or(0, |log| log.cells.iter().filter(|c| **c == cell).count())
}


#[test]
fn producer_suppresses_in_radius_opposing_ganger_only() {
    let mut app = battle_app(1);
    with_applied_log(&mut app);

    let target = ground(8, 5);
    let in_radius_player = target; 
    let out_of_radius_player = ground(20, 20);
    let same_faction_near = ground(8, 6); 
    let situation = SituationBuilder::new()
        .with_gangers([
            ganger(in_radius_player, PLAYER, Direction::West),
            ganger(out_of_radius_player, PLAYER, Direction::West),
            ganger(ground(4, 5), ENEMY, Direction::East),
            ganger(same_faction_near, ENEMY, Direction::East),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let players = gangers_of(&mut app, PLAYER);
    let enemies = gangers_of(&mut app, ENEMY);
    assert_eq!(players.len(), 2, "two players spawned");
    assert_eq!(enemies.len(), 2, "two enemies spawned");
    let Some(shooter) = ganger_of(&mut app, ENEMY) else {
        unreachable!("an enemy shooter spawned");
    };
    let in_player = players
        .iter()
        .copied()
        .find(|e| app.world().get::<Position>(*e).map(|p| **p) == Some(in_radius_player));
    let out_player = players
        .iter()
        .copied()
        .find(|e| app.world().get::<Position>(*e).map(|p| **p) == Some(out_of_radius_player));
    let same_faction = enemies
        .iter()
        .copied()
        .find(|e| app.world().get::<Position>(*e).map(|p| **p) == Some(same_faction_near));
    let (Some(in_player), Some(out_player), Some(same_faction)) =
        (in_player, out_player, same_faction)
    else {
        unreachable!("the three tagged gangers resolve by position");
    };

    let mode = wielded_single_mode(&mut app, shooter);
    app.world_mut().write_message(FireRequested::new(
        shooter,
        mode,
        Cell::new(8, 5),
        Level::new(0),
    ));
    step(&mut app, 3);

    assert!(
        is_suppressed(&app, in_player),
        "(a) the IN-radius opposing player is Suppressed",
    );
    assert!(
        !is_suppressed(&app, out_player),
        "(a) the OUT-of-radius opposing player is NOT suppressed",
    );
    assert!(
        !is_suppressed(&app, same_faction),
        "(a) a SAME-faction ganger (the shooter's own gang) is NEVER suppressed",
    );
    assert!(
        !is_suppressed(&app, shooter),
        "(a) the shooter never suppresses itself",
    );
    assert!(
        applied_carried_ganger(&app, in_player),
        "the SuppressionApplied signal must carry the freshly-pinned ganger entity",
    );
}


#[test]
fn idempotent_refresh_emits_suppression_applied_once() {
    let mut app = battle_app(1);
    with_applied_log(&mut app);
    let target = ground(8, 5);
    let situation = SituationBuilder::new()
        .with_gangers([
            ganger(target, PLAYER, Direction::West),
            ganger(ground(4, 5), ENEMY, Direction::East),
            ganger(ground(4, 6), ENEMY, Direction::East),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);
    let Some(player) = ganger_of(&mut app, PLAYER) else {
        unreachable!("a player spawned");
    };
    let enemies = gangers_of(&mut app, ENEMY);
    let (Some(&first_shooter), Some(&second_shooter)) = (enemies.first(), enemies.get(1)) else {
        unreachable!("two enemy shooters spawned");
    };

    let mode1 = wielded_single_mode(&mut app, first_shooter);
    app.world_mut().write_message(FireRequested::new(
        first_shooter,
        mode1,
        Cell::new(8, 5),
        Level::new(0),
    ));
    step(&mut app, 3);
    assert!(
        is_suppressed(&app, player),
        "(e) precondition: the first shot suppresses the player",
    );

    let mode2 = wielded_single_mode(&mut app, second_shooter);
    app.world_mut().write_message(FireRequested::new(
        second_shooter,
        mode2,
        Cell::new(8, 5),
        Level::new(0),
    ));
    step(&mut app, 3);

    assert!(
        is_suppressed(&app, player),
        "(e) the player is still suppressed after the refresh",
    );
    assert_eq!(
        applied_count_for(&app, target),
        1,
        "(e) SuppressionApplied fired exactly ONCE for the player's cell (a re-application of \
         an already-suppressed unit is an idempotent refresh — no second FCT signal)",
    );
}
