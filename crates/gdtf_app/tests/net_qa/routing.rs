//! Headless routing tests: the REAL [`route_requests`] system + deadline sweep, driven
//! through [`NetQaPlugin::with_channels`] against a `GdtfTestAppBuilder` app (GTW-736).

use std::sync::mpsc;

use bevy::app::App;
use gdtf_app::test_support::{
    AppState, NET_QA_PROTOCOL_VERSION, NET_QA_SERVER_NAME, NetQaPlugin, QaShotDir, ShotPollBudget,
    net_qa_hello_facts,
};
use gdtf_net_qa_transport::{IncomingRequest, Responder};
use gdtf_qa_protocol::{
    envelope::{QaError, QaRequest, QaResponse, ScreenshotResult},
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

/// GTW-940: the game's handshake facts are the ONE place its identity is stated — the version
/// it speaks paired with its own server name — and they are what
/// [`NetQaPlugin`](gdtf_app::test_support::NetQaPlugin) hands
/// [`run_listener`](gdtf_net_qa_transport::run_listener).
///
/// The negotiation itself is the TRANSPORT's, proven over a real socket in
/// `crates/gdtf_net_qa_transport/src/listener/test/session.rs` (which passes deliberately
/// non-default facts, so it proves the listener answers with whatever facts it was given).
/// What this host owes is that the facts it hands over are its own.
#[test]
fn the_hosts_hello_facts_are_its_own_version_and_name() {
    let facts = net_qa_hello_facts();
    assert_eq!(
        facts.protocol, NET_QA_PROTOCOL_VERSION,
        "the game must negotiate the version it declares it speaks",
    );
    assert_eq!(
        *facts.server, NET_QA_SERVER_NAME,
        "the game must identify itself by its own server name, so a client can tell which \
         host it reached",
    );
}

/// GTW-940: the router no longer negotiates. A [`Hello`](QaRequest::Hello) is answered in the
/// listener thread and never reaches the inbox — so the duplicated `answer_hello` is gone, and
/// one that somehow bypassed the listener is refused rather than quietly answered here.
#[test]
fn the_router_no_longer_answers_hello() {
    let (mut app, tx) = build_router_app();
    let reply = send(&tx, QaRequest::Hello(NET_QA_PROTOCOL_VERSION));
    app.update();
    let reply = reply.try_recv();
    assert!(
        matches!(&reply, Ok(QaResponse::Error(QaError::BadRequest))),
        "a Hello that reached the router must NOT be negotiated here — the listener thread \
         owns the handshake — got {reply:?}",
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
