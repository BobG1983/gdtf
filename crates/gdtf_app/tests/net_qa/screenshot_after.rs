//! GTW-749 — the T15 `screenshot_after` compound, driven through the REAL
//! `claim_screenshot_after` + `tick_after_shots` systems on a live-battle
//! `BattleAppBuilder` app (never a shadow copy).
//!
//! Three facts are pinned:
//!
//! - the embedded intent is injected through the SAME input queue the T4 `apply_injects`
//!   pump uses (a probed `MoveRequested` drains the same frame the request is claimed);
//! - a REJECTED embedded intent takes NO capture — ever — and answers with the typed
//!   `ScreenshotAfterResult::Rejected`, not a stale or wrong-moment PNG;
//! - an ACCEPTED intent's capture fires exactly `frame_delay` frames after the claim
//!   frame — a deterministic frame-count assertion, never a sleep/timing race.

use bevy::render::view::window::screenshot::Screenshot;
use gdtf_battle_sim::acts::MoveRequested;
use gdtf_qa_protocol::{
    envelope::{QaRequest, QaResponse, RejectReason, ScreenshotAfterResult},
    ids::{FrameDelay, GangerToken},
    intent::NetIntent,
};

use crate::inject_support::*;

/// Count the `Screenshot` marker entities present — a capture was SPAWNED iff this is
/// non-zero. No GPU/render plugin is present in this headless harness, so a spawned
/// `Screenshot` entity never resolves further (no despawn, no write) — a stable,
/// deterministic witness that a capture was (or was not) triggered, independent of
/// whether a PNG could ever land.
fn count_screenshot_entities(app: &mut bevy::app::App) -> usize {
    let world = app.world_mut();
    let mut q = world.query::<&Screenshot>();
    q.iter(world).count()
}

/// The embedded intent is pushed through the SAME `PendingActIntent` queue a bare
/// `Inject` uses: a probed `MoveRequested` drains the SAME frame the `ScreenshotAfter`
/// is claimed — the co-schedule same-frame guarantee, exactly as GTW-737 proves for a
/// bare `Inject`.
#[test]
fn screenshot_after_injects_its_intent_through_the_same_queue_the_same_frame() {
    let Some((mut app, tx)) = inject_battle_app() else {
        return;
    };
    add_classic_probe::<MoveRequested>(&mut app);
    let actor = spawn_actor(&mut app, 5, 5, 0);
    app.update();
    clear_probe::<MoveRequested>(&mut app);

    let reply = send(
        &tx,
        QaRequest::ScreenshotAfter {
            intent:      NetIntent::Move {
                dest: cell_level_net(6, 5, 0),
            },
            frame_delay: FrameDelay::new(10),
            name:        None,
        },
    );
    app.update();

    // No reply yet — the capture is deferred, not answered on the claim frame.
    assert!(
        reply.try_recv().is_err(),
        "a ScreenshotAfter with a nonzero frame_delay must not reply on the claim frame",
    );
    let emitted = gdtf_test_utils::probed::<MoveRequested>(&app);
    assert_eq!(
        emitted.len(),
        1,
        "the embedded Move must drain to exactly one MoveRequested the same frame it \
         is claimed",
    );
    assert_eq!(
        emitted[0].actor, actor,
        "the drained MoveRequested acts for the SelectedShooter",
    );
    assert_eq!(
        count_screenshot_entities(&mut app),
        0,
        "no capture may be spawned before the frame_delay elapses",
    );
}

/// A `ScreenshotAfter` whose embedded intent is REJECTED (a malformed/unknown token,
/// fail-closed exactly like a bare `Inject`) answers `ScreenshotAfterResult::Rejected`
/// immediately and spawns NO capture, ever — not now, and not after the `frame_delay`
/// window it would have waited had the intent queued.
#[test]
fn screenshot_after_with_a_rejected_intent_takes_no_capture() {
    let Some((mut app, tx)) = inject_battle_app() else {
        return;
    };
    spawn_actor(&mut app, 5, 5, 0);
    app.update();

    let reply = send(
        &tx,
        QaRequest::ScreenshotAfter {
            intent:      NetIntent::Stabilize {
                target: GangerToken::new(u64::MAX),
            },
            frame_delay: FrameDelay::new(5),
            name:        None,
        },
    );
    app.update();

    assert!(
        matches!(
            reply.try_recv(),
            Ok(QaResponse::ScreenshotAfter(
                ScreenshotAfterResult::Rejected(RejectReason::UnknownEntity)
            ))
        ),
        "a malformed token embedded in a ScreenshotAfter must be Rejected(UnknownEntity), \
         answered immediately",
    );
    // Drive well past the requested frame_delay: a rejected intent must NEVER fire a
    // capture, no matter how long the harness keeps ticking.
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(
        count_screenshot_entities(&mut app),
        0,
        "a rejected ScreenshotAfter must spawn no capture at any point — never a shot \
         of the wrong moment",
    );
}

/// The frame-delay window this test requests — arbitrary but small (fast test, ample
/// distance from the claim frame to prove the "not yet" window).
const DELAY: u32 = 3;

/// FRAME-EXACT capture: an accepted `ScreenshotAfter`'s capture is spawned exactly
/// `frame_delay` frames after the claim frame — asserted by counting `app.update()`
/// calls and observing the `Screenshot` marker entity's spawn moment, never a
/// sleep/timing race.
#[test]
fn screenshot_after_fires_the_capture_exactly_on_the_requested_frame_delta() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        return;
    };
    let Some((mut app, tx)) = inject_battle_app() else {
        return;
    };
    // The T7 pump's confinement dir + poll budget, overridden exactly like the T7
    // integration suite: a temp dir (no tree artifact) + budget is irrelevant here (the
    // frame-delta assertion is about the CLAIM/tick timing, not the poll timeout).
    app.insert_resource(gdtf_app::test_support::QaShotDir::new(
        tmp.path().to_path_buf(),
    ));
    app.insert_resource(gdtf_app::test_support::ShotPollBudget::new(2));
    spawn_actor(&mut app, 5, 5, 0);
    app.update();

    let reply = send(
        &tx,
        QaRequest::ScreenshotAfter {
            intent:      NetIntent::EndTurn,
            frame_delay: FrameDelay::new(DELAY),
            name:        None,
        },
    );
    // The claim frame: the intent queues, the capture is NOT yet due.
    app.update();
    assert_eq!(
        count_screenshot_entities(&mut app),
        0,
        "no capture may be spawned on the claim frame itself",
    );

    // Every frame within the delay window: still not due.
    for tick in 0..DELAY {
        app.update();
        assert_eq!(
            count_screenshot_entities(&mut app),
            0,
            "no capture may be spawned before its frame_delay elapses (tick {tick})",
        );
    }

    // The very next frame: the countdown elapses and the capture fires.
    app.update();
    assert_eq!(
        count_screenshot_entities(&mut app),
        1,
        "the capture must fire exactly on the requested frame delta",
    );
    // No reply yet from the claim itself (a `ScreenshotAfterResult::Rejected` never
    // arrives for an accepted intent) — the poll/Saved-or-TimedOut reply is a separate,
    // later concern the T7 pump's own budget governs (proven by the shared T7 suite).
    assert!(
        !matches!(
            reply.try_recv(),
            Ok(QaResponse::ScreenshotAfter(
                ScreenshotAfterResult::Rejected(_)
            ))
        ),
        "an accepted ScreenshotAfter must never answer Rejected",
    );
}
