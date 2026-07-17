//! GTW-737 — the T4 inject path, driven through the REAL `apply_injects` system on a
//! live-battle `BattleAppBuilder` app (never a shadow copy).
//!
//! Each test injects a request through the same inbox the loopback listener feeds, drives
//! ONE `app.update()`, and reads the honest receipt off the reply channel plus the
//! downstream drain's emission — proving the co-schedule same-frame guarantee (route →
//! apply → drain in one tick), the fail-closed token resolution, the offer gate, and the
//! receipt's decoupling from the act's outcome.

use gdtf_battle_sim::acts::{MoveRequested, StabilizeDownedRequested};
use gdtf_qa_protocol::{
    envelope::{InjectReceipt, QaRequest, QaResponse, RejectReason},
    ids::GangerToken,
    intent::NetIntent,
};
use gdtf_test_utils::probed;

use crate::inject_support::*;

/// SAME-FRAME (classic): an injected `Move` is pushed onto `PendingActIntent` by the pump
/// and drained by `dispatch_act_intents` — emitting exactly one `MoveRequested` for the
/// selection — the SAME `app.update()` the request is routed in. The receipt is `Queued`.
#[test]
fn injected_move_is_drained_the_same_frame() {
    let Some((mut app, tx)) = inject_battle_app() else {
        return;
    };
    add_classic_probe::<MoveRequested>(&mut app);
    let actor = spawn_actor(&mut app, 5, 5, 0);
    app.update();
    clear_probe::<MoveRequested>(&mut app);

    let reply = send(
        &tx,
        QaRequest::Inject(NetIntent::Move {
            dest: cell_level_net(6, 5, 0),
        }),
    );
    app.update();

    assert!(
        matches!(
            reply.try_recv(),
            Ok(QaResponse::Injected(InjectReceipt::Queued))
        ),
        "an injected Move must be Queued the frame it enters the input queue",
    );
    let emitted = probed::<MoveRequested>(&app);
    assert_eq!(
        emitted.len(),
        1,
        "the injected Move must drain to exactly one MoveRequested the same frame",
    );
    assert_eq!(
        emitted[0].actor, actor,
        "the drained MoveRequested acts for the SelectedShooter",
    );
}

/// SAME-FRAME (contextual): with a Stabilize target OFFERED (an 8-adjacent bleeding downed
/// ally), an injected `Stabilize` is pushed onto `PendingContextualIntents<StabilizeAct>`
/// and drained by the act's generic drain — emitting exactly one `StabilizeDownedRequested`
/// — the SAME update. The receipt is `Queued`.
#[test]
fn injected_offered_stabilize_is_drained_the_same_frame() {
    let Some((mut app, tx)) = inject_battle_app() else {
        return;
    };
    add_contextual_probe::<StabilizeDownedRequested>(&mut app);
    let actor = spawn_actor(&mut app, 5, 5, 0);
    let target = spawn_downed_ally(&mut app, 5, 6, 0);
    // First update: the offer scan fills ContextualOffer<StabilizeAct> = Some(target).
    app.update();
    clear_probe::<StabilizeDownedRequested>(&mut app);

    let reply = send(
        &tx,
        QaRequest::Inject(NetIntent::Stabilize {
            target: GangerToken::new(target.to_bits()),
        }),
    );
    app.update();

    assert!(
        matches!(
            reply.try_recv(),
            Ok(QaResponse::Injected(InjectReceipt::Queued))
        ),
        "an offered Stabilize inject must be Queued the same frame",
    );
    let emitted = probed::<StabilizeDownedRequested>(&app);
    assert_eq!(
        emitted.len(),
        1,
        "the injected Stabilize must drain to exactly one StabilizeDownedRequested the same frame",
    );
    assert_eq!(emitted[0].actor, actor, "the actor is the SelectedShooter");
    assert_eq!(
        emitted[0].target, target,
        "the target is the offered downed ally",
    );
}

/// OFFER GATE: an injected contextual act whose target is a live entity but NOT currently
/// offered is `Rejected(NotOffered)` — and nothing reaches the contextual queue (no
/// downstream drain emission), never a silent push.
#[test]
fn injected_unoffered_contextual_is_rejected_not_offered() {
    let Some((mut app, tx)) = inject_battle_app() else {
        return;
    };
    add_contextual_probe::<StabilizeDownedRequested>(&mut app);
    spawn_actor(&mut app, 5, 5, 0);
    // A live, resolvable downed ally FAR from the actor — so it is never OFFERED.
    let far_ally = spawn_downed_ally(&mut app, 20, 20, 0);
    app.update();
    clear_probe::<StabilizeDownedRequested>(&mut app);

    let reply = send(
        &tx,
        QaRequest::Inject(NetIntent::Stabilize {
            target: GangerToken::new(far_ally.to_bits()),
        }),
    );
    app.update();

    assert!(
        matches!(
            reply.try_recv(),
            Ok(QaResponse::Injected(InjectReceipt::Rejected(
                RejectReason::NotOffered
            )))
        ),
        "a Stabilize on an un-offered target must be Rejected(NotOffered)",
    );
    assert!(
        probed::<StabilizeDownedRequested>(&app).is_empty(),
        "an un-offered inject must push nothing to the contextual queue (no drained request)",
    );
}

/// FAIL-CLOSED tokens: a DEAD (despawned) token and a MALFORMED bit pattern each resolve
/// to `Rejected(UnknownEntity)` — never a panic (proving the runtime behaviour, not just
/// the absence of the panic lint).
#[test]
fn injected_dead_or_malformed_token_is_rejected_unknown_entity() {
    let Some((mut app, tx)) = inject_battle_app() else {
        return;
    };
    spawn_actor(&mut app, 5, 5, 0);
    // A DEAD token: spawn a ganger, capture its bits, then despawn it.
    let doomed = spawn_downed_ally(&mut app, 6, 6, 0);
    let dead_bits = doomed.to_bits();
    app.world_mut().despawn(doomed);
    app.update();

    let dead_reply = send(
        &tx,
        QaRequest::Inject(NetIntent::Stabilize {
            target: GangerToken::new(dead_bits),
        }),
    );
    // A MALFORMED token: a bit pattern no live Entity carries.
    let malformed_reply = send(
        &tx,
        QaRequest::Inject(NetIntent::Stabilize {
            target: GangerToken::new(u64::MAX),
        }),
    );
    app.update();

    assert!(
        matches!(
            dead_reply.try_recv(),
            Ok(QaResponse::Injected(InjectReceipt::Rejected(
                RejectReason::UnknownEntity
            )))
        ),
        "a despawned (dead) token must be Rejected(UnknownEntity), never a panic",
    );
    assert!(
        matches!(
            malformed_reply.try_recv(),
            Ok(QaResponse::Injected(InjectReceipt::Rejected(
                RejectReason::UnknownEntity
            )))
        ),
        "a malformed token must be Rejected(UnknownEntity), never a panic",
    );
}

/// RECEIPT HONESTY: a Move whose destination is out of bounds (the sim will reject the
/// move) is STILL `Queued` the same frame — the receipt reports only that the intent
/// entered the input queue, decoupled from the act's outcome (we read ONLY the reply channel,
/// never a downstream move result).
#[test]
fn injected_receipt_is_queued_even_when_the_act_cannot_succeed() {
    let Some((mut app, tx)) = inject_battle_app() else {
        return;
    };
    spawn_actor(&mut app, 5, 5, 0);
    app.update();

    let reply = send(
        &tx,
        QaRequest::Inject(NetIntent::Move {
            dest: cell_level_net(9_999, 9_999, 0),
        }),
    );
    app.update();

    assert!(
        matches!(
            reply.try_recv(),
            Ok(QaResponse::Injected(InjectReceipt::Queued))
        ),
        "the receipt must be Queued the same frame regardless of whether the move can succeed",
    );
}
