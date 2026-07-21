//! T13 / T14 / T15 — the drawn world lags the sim, and recovers from a gap.
//!
//! These two are the tests the whole ticket turns on. It is entirely possible to ship a
//! beautifully paced combat LOG while the ganger sprites still snap to their new state the
//! frame the sim resolves it — a reactor that turns to face its target instantly while its
//! victim vanishes at drain time is the reported symptom, unfixed. What stops that is
//! asserting on the components the sprite systems actually read.

use std::time::Duration;

use gdtf_battle_sim::{
    act_log::{
        ActDeed, ActLog, ActLogCapacity, ActProvenance, PoseFacts, PositionFacts, SuppressedNow,
    },
    ganger::{
        Aiming, Direction, Facing, Hp, LifeState, Position, Stance, StanceKind, Suppressed,
        SuppressorCell,
    },
};

use super::harness::*;

/// **T13 — THE SPRITE LAGS THE SIM.** A ganger's drawn position must stay at its OLD cell
/// while the step that moved it is still unplayed, and move only when that step is shown.
///
/// `move_ganger_sprites` retargets on `Changed<DrawnPosition>`. So this is the sprite-level
/// assertion in its exact mechanical form: while the entry is unshown, `DrawnPosition` is
/// unchanged and the sprite does not move; the frame the entry is released, `DrawnPosition`
/// changes and the tween retargets. If the mirror were skipped and the mover left on
/// `Changed<Position>`, the sprite would jump the instant the sim stepped — a fully green
/// log-and-cursor suite with the reported symptom still on screen.
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

    // The SIM moves. Two entries: a slow beat first, then the step — so the step is
    // genuinely unplayed for a while, exactly as it is behind a reaction volley.
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

    // Play only the FIRST entry, then hold.
    step(&mut app, Duration::from_millis(1));
    assert_eq!(*shown(&app), 1);

    // MID-HOLD: the sim has already moved; the DRAWN position has not.
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

    // Release the step. Now — and only now — the drawn position catches up.
    for _ in 0..6 {
        step(&mut app, Duration::from_millis(100));
        if *shown(&app) >= 2 {
            break;
        }
    }
    assert_eq!(*shown(&app), 2, "the step was shown");
    assert_eq!(
        drawn_position(&app, ganger),
        Some(Position::new(destination)),
        "once the step is shown, the drawn position moves — and that `Changed<DrawnPosition>` \
         is what retargets the sprite's glide",
    );
}

/// **T14 — THE POSE LAGS TOO, INCLUDING A SUPPRESSION CLEAR.** A facing change must not be
/// drawn before it is played, and clearing suppression must be observable at all.
///
/// Two properties in one test, because they are the same mechanism:
///
/// * A reactor turning to face its target is the single most visible tell of who is
///   shooting. Under the old wiring it snapped the frame the sim resolved the turn — while
///   the tracers were still animating — which is the reported symptom.
/// * Suppression CLEARING used to be a component removal, which change detection cannot
///   see, so the appearance resolver had to drain `RemovedComponents<Suppressed>`
///   separately. Carrying suppression as a FIELD of the drawn pose retires that path
///   entirely: a clear is now an ordinary value change.
#[test]
fn drawn_pose_lags_a_facing_change_and_a_suppression_clear() {
    let mut app = playback_app();
    let ganger = spawn_ganger(&mut app, ground(1, 1), Direction::North);
    // Start SUPPRESSED, so the clear below is a real transition.
    app.world_mut()
        .entity_mut(ganger)
        .insert(Suppressed::new(SuppressorCell::new(ground(9, 9))));
    seed(&mut app);

    let seeded = drawn_pose(&app, ganger);
    assert!(
        seeded.is_some_and(drawn_suppressed),
        "the mirror seeds from live state, so a suppressed ganger starts drawn suppressed",
    );

    // The SIM turns to face east AND drops the suppression, both at once.
    if let Some(mut facing) = app.world_mut().get_mut::<Facing>(ganger) {
        *facing = Facing::new(Direction::East);
    }
    app.world_mut().entity_mut(ganger).remove::<Suppressed>();

    // Behind a slow entry, so the pose change is genuinely unplayed for a while.
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

    // MID-HOLD: the sim already faces east and is no longer suppressed; the drawn pose is
    // still the old one, on BOTH counts.
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

    // Release it.
    for _ in 0..6 {
        step(&mut app, Duration::from_millis(100));
        if *shown(&app) >= 2 {
            break;
        }
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

/// **T15 — a cursor that falls off the window recovers.** When entries the cursor never
/// showed have been evicted from the ring, it must resync the drawn world and jump — never
/// stall, and never rewind.
///
/// A gap only happens when playback has been starved for a very long time, at which point
/// the drawn world is wildly stale and replaying the retained tail one beat at a time would
/// take longer than the battle. So the cursor snaps forward to current truth and counts what
/// it skipped: a degradation, but a loud and monotone one.
#[test]
fn a_cursor_that_falls_off_the_window_jumps_and_applies_skipped_drawn_state() {
    let mut app = playback_app();
    let start = ground(0, 0);
    let ganger = spawn_ganger(&mut app, start, Direction::North);
    seed(&mut app);

    // A tiny ring, so overflow is reachable in a few appends.
    app.insert_resource(ActLog::new(ActLogCapacity::new(2)));

    // The sim races ahead: it moves and is downed, and appends far more entries than the
    // ring can hold — so the early ones are evicted before the cursor ever reaches them.
    let destination = ground(7, 7);
    if let Some(mut position) = app.world_mut().get_mut::<Position>(ganger) {
        *position = Position::new(destination);
    }
    if let Some(mut life) = app.world_mut().get_mut::<LifeState>(ganger) {
        *life = LifeState::Downed;
    }
    if let Some(mut hp) = app.world_mut().get_mut::<Hp>(ganger) {
        *hp = Hp::new(1);
    }
    for _ in 0..8 {
        append(
            &mut app,
            ganger,
            ActProvenance::Commanded,
            ActDeed::BleedStarted,
        );
    }

    let head = {
        let Some(log) = app.world().get_resource::<ActLog>() else {
            unreachable!("the fixture inserts an act log");
        };
        assert!(
            *log.dropped() > 0,
            "the fixture must actually overflow the ring, else the gap path is untested",
        );
        log.head()
    };

    step(&mut app, Duration::from_millis(1));

    assert_eq!(
        shown(&app),
        head,
        "a cursor that fell off the window jumps to the head — it never stalls, and it only \
         ever moves forward, so it cannot render a false sequence by rewinding",
    );
    assert!(
        *app.world()
            .resource::<gdtf_battle_presenter::PlaybackCursor>()
            .skipped()
            > 0,
        "the skip is COUNTED — the degradation is loud, not silent",
    );
    // And the drawn world was resynced to live truth rather than left stale.
    assert_eq!(
        drawn_position(&app, ganger),
        Some(Position::new(destination)),
        "the gap recovery resyncs the drawn position",
    );
    assert_eq!(
        drawn_life(&app, ganger),
        Some(LifeState::Downed),
        "the gap recovery resyncs the drawn life state",
    );
    assert_eq!(
        drawn_hp(&app, ganger),
        Some(Hp::new(1)),
        "the gap recovery resyncs the drawn vitals",
    );
}
