//! GTW-816: the wire half of the DEV UI-stack swap harness, on a live-battle app with the
//! REAL `net_qa` router + inject pump and the REAL swap harness.
//!
//! Three things are pinned here:
//!
//! 1. an injected `SwapUiStack` flips the live stack — the SAME state the keyboard shortcut
//!    flips, reached through the same latch;
//! 2. `GetBattleState` reports which stack is live (and which pair is under comparison);
//! 3. an injected `PressKey` of the harness's shortcut key flips it too — the wire mirror of
//!    the developer's own keystroke, which is what lets an agent capture the KEYBOARD swap
//!    in the running game rather than only the intent-driven one.
//!
//! The battle app is `MinimalPlugins`, so the harness's egui DRAW is not registered (egui
//! needs the render stack — see the harness plugin's doc); everything asserted here is the
//! swap state and the wire path, neither of which depends on the draw. The RENDERING half
//! of the proof, and the cross-stack behavioural-parity script (which needs a real egui
//! pass to mean anything), live in the `ui_swap` suite on a `DefaultPlugins` app.

use std::sync::mpsc;

use bevy::prelude::*;
use gdtf_app::test_support::{IncomingRequest, UiStack, UiStackId, UiSwapHarnessPlugin};
use gdtf_qa_protocol::{
    envelope::{InjectReceipt, QaRequest, QaResponse},
    intent::{KeyNet, KeyPressNet, NetIntent, UiStackNet},
    view::{BattleView, UiStackView},
};
use gdtf_test_utils::BattleAppBuilder;

use super::inject_support::send;

/// Build a live-battle app with the REAL `net_qa` channel AND the REAL swap harness wired.
///
/// Returns `None` if the battle drive never rests, so the caller asserts the `Some` and this
/// helper stays panic-free.
fn swap_battle_app() -> Option<(App, mpsc::Sender<IncomingRequest>)> {
    let mut app = BattleAppBuilder::new().build()?;
    let (tx, rx) = mpsc::channel();
    app.add_plugins(gdtf_app::test_support::NetQaPlugin::with_channels(rx));
    app.add_plugins(UiSwapHarnessPlugin);
    // Settle the freshly-added router, pump and harness into the running battle.
    for _ in 0..2 {
        app.update();
    }
    Some((app, tx))
}

/// The live stack the harness is holding.
fn live_stack(app: &App) -> Option<UiStackId> {
    app.world().get_resource::<UiStack>().map(|s| s.live())
}

/// Inject one intent and settle the frame it is drained on.
fn inject(app: &mut App, tx: &mpsc::Sender<IncomingRequest>, intent: NetIntent) -> QaResponse {
    let reply = send(tx, QaRequest::Inject(intent));
    app.update();
    reply
        .try_recv()
        .unwrap_or(QaResponse::Injected(InjectReceipt::Rejected(
            gdtf_qa_protocol::envelope::RejectReason::NoBattle,
        )))
}

/// Ask for a fresh battle snapshot.
fn snapshot(app: &mut App, tx: &mpsc::Sender<IncomingRequest>) -> Option<BattleView> {
    let reply = send(tx, QaRequest::GetBattleState);
    app.update();
    match reply.try_recv() {
        Ok(QaResponse::Battle(view)) => Some(view),
        _ => None,
    }
}

/// An injected `SwapUiStack` flips the live stack — the same state, through the same latch,
/// that the keyboard shortcut flips.
///
/// Absolute, not a flip: naming `Egui` twice leaves `Egui` live, which is what lets an agent
/// put the harness in a known configuration without reading it first.
#[test]
fn the_swap_intent_flips_the_live_stack_headlessly() -> Result<(), &'static str> {
    let (mut app, tx) = swap_battle_app().ok_or("the battle drive must rest")?;
    assert_eq!(live_stack(&app), Some(UiStackId::BevyUi), "the default");

    let receipt = inject(
        &mut app,
        &tx,
        NetIntent::SwapUiStack {
            stack: UiStackNet::Egui,
        },
    );
    assert!(
        matches!(receipt, QaResponse::Injected(InjectReceipt::Queued)),
        "the swap intent must be accepted; got {receipt:?}",
    );
    app.update();
    assert_eq!(
        live_stack(&app),
        Some(UiStackId::Egui),
        "the wire swap must reach the same live stack the keyboard reaches",
    );

    // Absolute, so a repeat is a no-op rather than a flip back.
    inject(
        &mut app,
        &tx,
        NetIntent::SwapUiStack {
            stack: UiStackNet::Egui,
        },
    );
    app.update();
    assert_eq!(live_stack(&app), Some(UiStackId::Egui), "still Egui");

    inject(
        &mut app,
        &tx,
        NetIntent::SwapUiStack {
            stack: UiStackNet::BevyUi,
        },
    );
    app.update();
    assert_eq!(
        live_stack(&app),
        Some(UiStackId::BevyUi),
        "and naming the other stack swaps back",
    );
    Ok(())
}

/// The `query_state` snapshot reports which stack is active, and which pair is under
/// comparison — before AND after a swap, so an agent can name the configuration each of its
/// parity assertions was made in.
#[test]
fn the_snapshot_reports_the_active_stack() -> Result<(), &'static str> {
    let (mut app, tx) = swap_battle_app().ok_or("the battle drive must rest")?;

    let before = snapshot(&mut app, &tx).ok_or("the snapshot service must answer")?;
    let before: UiStackView = before
        .ui_stack
        .ok_or("a build WITH the harness must report its stack")?;
    assert_eq!(before.live, UiStackNet::BevyUi);
    assert_eq!(before.comparison.baseline, UiStackNet::BevyUi);
    assert_eq!(before.comparison.candidate, UiStackNet::Egui);

    inject(
        &mut app,
        &tx,
        NetIntent::SwapUiStack {
            stack: UiStackNet::Egui,
        },
    );
    app.update();

    let after = snapshot(&mut app, &tx).ok_or("the snapshot service must answer")?;
    let after: UiStackView = after.ui_stack.ok_or("still a harness build")?;
    assert_eq!(
        after.live,
        UiStackNet::Egui,
        "the snapshot must track the swap, not report a stale stack",
    );
    assert_eq!(
        after.comparison, before.comparison,
        "the compared pair does not change when the live one does",
    );
    Ok(())
}

/// An injected keypress of the harness's SHORTCUT KEY swaps the stack — the wire mirror of
/// a developer's own F9, resolved through the real `KeyNet` → `KeyCode` mapping and folded
/// into `ButtonInput<KeyCode>` by Bevy's own `keyboard_input_system`.
///
/// This is the route an agent uses to drive (and screenshot) the KEYBOARD half of the
/// harness in the running game: without the key in the wire vocabulary the shortcut is
/// developer-only and no headless client can exercise it.
#[test]
fn an_injected_shortcut_keypress_flips_the_live_stack() -> Result<(), &'static str> {
    let (mut app, tx) = swap_battle_app().ok_or("the battle drive must rest")?;
    assert_eq!(live_stack(&app), Some(UiStackId::BevyUi), "the default");

    let receipt = inject(
        &mut app,
        &tx,
        NetIntent::PressKey {
            key: KeyPressNet::Key(KeyNet::F9),
        },
    );
    assert!(
        matches!(receipt, QaResponse::Injected(InjectReceipt::Queued)),
        "the keypress must be accepted; got {receipt:?}",
    );
    // One frame for `keyboard_input_system` to fold the press in and the harness to apply it.
    app.update();

    assert_eq!(
        live_stack(&app),
        Some(UiStackId::Egui),
        "the injected shortcut key must reach the same live stack the wire intent reaches",
    );
    Ok(())
}
