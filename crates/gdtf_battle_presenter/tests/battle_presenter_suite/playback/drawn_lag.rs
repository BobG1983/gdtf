use std::time::Duration;

use gdtf_battle_sim::{
    act_log::{ActDeed, ActProvenance, PoseFacts, PositionFacts, SuppressedNow},
    ganger::{Aiming, Direction, Facing, Position, Stance, StanceKind, Suppressed, SuppressorCell},
};

use super::harness::*;

#[test]
fn the_drawn_position_lags_the_sim_position_until_the_step_plays() {
    let mut app = playback_app();
    let start = ground(2, 2);
    let ganger = spawn_ganger(&mut app, start, Direction::North);
    seed(&mut app);

    assert_eq!(
        drawn_position(&app, ganger),
        Some(Position::new(start)),
        "the mirror is seeded equal to the sim at spawn — they only diverge once the sim acts",
    );

    let destination = ground(5, 2);
    if let Some(mut position) = app.world_mut().get_mut::<Position>(ganger) {
        *position = Position::new(destination);
    }
    append(
        &mut app,
        ganger,
        ActProvenance::Commanded,
        ActDeed::BleedStarted,
    );
    append(
        &mut app,
        ganger,
        ActProvenance::Commanded,
        ActDeed::MovedTo {
            position: PositionFacts::new(Position::new(destination)),
        },
    );

    step(&mut app, Duration::from_millis(1));
    assert_eq!(*shown(&app), 1);

    assert_eq!(
        app.world().get::<Position>(ganger).copied(),
        Some(Position::new(destination)),
        "the sim moved immediately — it never waits",
    );
    assert_eq!(
        drawn_position(&app, ganger),
        Some(Position::new(start)),
        "the DRAWN position must still be the old cell while the step is unplayed — this is \
         the assertion that the sprite lags the sim, and it fails outright if the mirror is \
         skipped and the sprite mover is left reading live `Position`",
    );

    while *shown(&app) < 2 {
        step(&mut app, Duration::from_millis(100));
    }
    assert_eq!(*shown(&app), 2, "the step was shown");
    assert_eq!(
        drawn_position(&app, ganger),
        Some(Position::new(destination)),
        "once the step is shown, the drawn position moves — and that `Changed<DrawnPosition>` \
         is what retargets the sprite's glide",
    );
}

#[test]
fn drawn_pose_lags_a_facing_change_and_a_suppression_clear() {
    let mut app = playback_app();
    let ganger = spawn_ganger(&mut app, ground(1, 1), Direction::North);
    app.world_mut()
        .entity_mut(ganger)
        .insert(Suppressed::new(SuppressorCell::new(ground(9, 9))));
    seed(&mut app);

    let seeded = drawn_pose(&app, ganger);
    assert!(
        seeded.is_some_and(drawn_suppressed),
        "the mirror seeds from live state, so a suppressed ganger starts drawn suppressed",
    );

    if let Some(mut facing) = app.world_mut().get_mut::<Facing>(ganger) {
        *facing = Facing::new(Direction::East);
    }
    app.world_mut().entity_mut(ganger).remove::<Suppressed>();

    append(
        &mut app,
        ganger,
        ActProvenance::Commanded,
        ActDeed::BleedStarted,
    );
    append(
        &mut app,
        ganger,
        ActProvenance::Commanded,
        ActDeed::PostureChanged {
            pose: PoseFacts::new(
                Facing::new(Direction::East),
                Stance::new(StanceKind::Standing),
                Aiming::new(false),
                SuppressedNow::new(false),
            ),
        },
    );

    step(&mut app, Duration::from_millis(1));
    assert_eq!(*shown(&app), 1);

    let mid = drawn_pose(&app, ganger);
    assert!(
        mid.is_some_and(|pose| *pose.facing() == Direction::North),
        "the DRAWN facing must still be the old one while the posture change is unplayed — \
         a reactor must not snap to its firing facing before its shot is shown",
    );
    assert!(
        mid.is_some_and(drawn_suppressed),
        "the DRAWN suppression must still be set while the clear is unplayed",
    );

    while *shown(&app) < 2 {
        step(&mut app, Duration::from_millis(100));
    }
    assert_eq!(*shown(&app), 2, "the posture change was shown");
    let after = drawn_pose(&app, ganger);
    assert!(
        after.is_some_and(|pose| *pose.facing() == Direction::East),
        "once shown, the drawn facing turns — and that change is what re-stamps the sprite",
    );
    assert!(
        after.is_some_and(|pose| !drawn_suppressed(pose)),
        "the suppression CLEAR is an ordinary field change on the drawn pose, so change \
         detection sees it — the `RemovedComponents<Suppressed>` drain is not needed at all",
    );
}
