//! Headless routing tests: the REAL [`route_requests`] system + deadline sweep, driven
//! through [`NetQaPlugin::with_channels`] against a `GdtfTestAppBuilder` app (GTW-736).

use std::sync::mpsc;

use bevy::app::App;
use gdtf_app::test_support::{
    AppState, NET_QA_PROTOCOL_VERSION, NetQaPlugin, QaShotDir, ShotPollBudget,
};
use gdtf_net_qa_transport::{IncomingRequest, Responder};
use gdtf_qa_protocol::{
    envelope::{ProtocolVersion, QaError, QaRequest, QaResponse, ScreenshotResult},
    view::AppStateNet,
};
use gdtf_test_utils::GdtfTestAppBuilder;

/// Build a headless app resting in [`AppState::Running`] with the REAL `net_qa` router
/// wired to an injected inbox, returning the app and the sender the test pushes requests
/// on (exactly as the listener thread would).
fn build_router_app() -> (App, mpsc::Sender<IncomingRequest>) {
    let (tx, rx) = mpsc::channel();
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();
    app.add_plugins(NetQaPlugin::with_channels(rx));
    // Settle into Running (and run one empty router pass).
    app.update();
    (app, tx)
}

/// Push a request onto the router's inbox and return the channel its reply arrives on.
fn send(tx: &mpsc::Sender<IncomingRequest>, request: QaRequest) -> mpsc::Receiver<QaResponse> {
    let (responder, reply_rx) = Responder::channel();
    let sent = tx.send(IncomingRequest::new(request, responder));
    assert!(sent.is_ok(), "the router inbox must be open");
    reply_rx
}

/// A [`Hello`](QaRequest::Hello) carrying the server's version negotiates successfully —
/// the reply is [`HelloOk`](QaResponse::HelloOk) with that exact version.
#[test]
fn hello_is_answered_with_the_server_version() {
    let (mut app, tx) = build_router_app();
    let reply = send(&tx, QaRequest::Hello(NET_QA_PROTOCOL_VERSION));
    app.update();
    let reply = reply.try_recv();
    assert!(
        matches!(
            &reply,
            Ok(QaResponse::HelloOk(facts)) if facts.protocol == NET_QA_PROTOCOL_VERSION
        ),
        "expected HelloOk echoing the negotiated version, got {reply:?}",
    );
}

/// A [`Hello`](QaRequest::Hello) carrying a foreign version is rejected
/// [`VersionMismatch`](QaError::VersionMismatch).
#[test]
fn hello_with_a_foreign_version_is_rejected() {
    let (mut app, tx) = build_router_app();
    // A version the server does not speak (its own + a large offset).
    let foreign = ProtocolVersion::new(*NET_QA_PROTOCOL_VERSION + 1000);
    let reply = send(&tx, QaRequest::Hello(foreign));
    app.update();
    let reply = reply.try_recv();
    assert!(
        matches!(&reply, Ok(QaResponse::Error(QaError::VersionMismatch))),
        "a foreign protocol version must be rejected VersionMismatch, got {reply:?}",
    );
}

/// [`GetAppFlow`](QaRequest::GetAppFlow) is answered OUTSIDE a battle — it needs no
/// `BattleInProgress` — and reports the lifecycle state with `battle_active == false`.
#[test]
fn get_app_flow_is_answered_outside_battle() {
    let (mut app, tx) = build_router_app();
    let reply = send(&tx, QaRequest::GetAppFlow);
    app.update();
    let reply = reply.try_recv();
    assert!(
        matches!(
            &reply,
            Ok(QaResponse::AppFlow(view))
                if view.state == AppStateNet::Running && !*view.battle_active
        ),
        "expected AppFlow(Running, battle_active=false), got {reply:?}",
    );
}

/// A battle-dependent request ([`GetBattleState`](QaRequest::GetBattleState)) with no
/// `BattleInProgress` resource is rejected [`NoBattle`](QaError::NoBattle) at ROUTE time —
/// the router never panics on the missing resource.
#[test]
fn battle_dependent_request_is_rejected_no_battle() {
    let (mut app, tx) = build_router_app();
    let reply = send(&tx, QaRequest::GetBattleState);
    app.update();
    let reply = reply.try_recv();
    assert!(
        matches!(&reply, Ok(QaResponse::Error(QaError::NoBattle))),
        "a battle-dependent request off-battle must be rejected NoBattle, got {reply:?}",
    );
}

/// A [`TakeScreenshot`](QaRequest::TakeScreenshot) routed through the LIVE plugin is claimed
/// by the T7 screenshot pump — NOT the generic deadline sweep (the screenshot queue has none)
/// — which runs its OWN multi-frame poll and, with no GPU to flush a PNG, answers the pump's
/// typed [`ScreenshotResult::TimedOut`] rather than the sweep's [`QaError::Timeout`]. Confined
/// to a temp directory + a tiny poll budget so it is fast and leaves no artifact in the tree.
/// This exercises the live `register_router` pump wiring: drop the pump + its resources from
/// it and this test fails (the request would be enqueued and never answered).
#[test]
fn take_screenshot_is_claimed_by_the_live_pump_and_times_out() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        return;
    };
    let (mut app, tx) = build_router_app();
    // Inject the pump's config Resources: a temp confinement dir (no tree artifact) + a tiny
    // budget (fast timeout) — exactly the Resources `register_router` inits, overridden here.
    app.insert_resource(QaShotDir::new(tmp.path().to_path_buf()));
    app.insert_resource(ShotPollBudget::new(2));
    let reply = send(&tx, QaRequest::TakeScreenshot { name: None });
    // The route enqueues on the first update; the pump claims it that frame and polls from the
    // next. With no GPU the PNG never lands, so the pump's own budget elapses to a timeout.
    for _ in 0..12 {
        app.update();
    }
    let reply = reply.try_recv();
    assert!(
        matches!(
            &reply,
            Ok(QaResponse::Screenshot(ScreenshotResult::TimedOut))
        ),
        "the live T7 pump must claim a routed TakeScreenshot and answer its own \
         ScreenshotResult::TimedOut, got {reply:?}",
    );
}
