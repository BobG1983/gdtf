//! GTW-746 — state-based affordance discovery: the `available` list `GetAppFlow`
//! advertises stays in agreement with what the REAL router actually accepts.
//!
//! Each test builds one state fixture (no battle, the main menu, a live battle), reads the
//! affordance list off a `GetAppFlow` reply, then probes the router with a request of every
//! [`RequestKindNet`] and asserts the two agree:
//!
//! - every kind the snapshot advertised as available is genuinely accepted by the router in
//!   that state (never answered [`QaError::NoBattle`](gdtf_qa_protocol::envelope::QaError));
//! - every kind it did NOT advertise is a battle-dependent kind that the router genuinely
//!   rejects [`QaError::NoBattle`] in that state.
//!
//! Because the assertion compares the wire-advertised set against the router's observed
//! behavior — two independent observations — it FAILS if a future request type's
//! accept/reject logic and its advertised-availability logic ever diverge, rather than
//! merely checking that some list was produced.

use std::sync::mpsc;

use bevy::app::App;
use gdtf_app::test_support::{AppState, IncomingRequest, NET_QA_PROTOCOL_VERSION, NetQaPlugin};
use gdtf_qa_protocol::{
    envelope::{QaError, QaRequest, QaResponse, StepperCommandNet},
    ids::{FrameDelay, SituationRef},
    intent::NetIntent,
    view::RequestKindNet,
};
use gdtf_test_utils::GdtfTestAppBuilder;

use crate::{
    inject_support::{inject_battle_app, send},
    start_battle::menu_app_with_net_qa,
};

/// Build a headless app in [`AppState::Running`] with no battle and the REAL router wired
/// to an injected inbox — the "no battle" fixture. Returns the app and the request sender.
fn no_battle_app() -> (App, mpsc::Sender<IncomingRequest>) {
    let (tx, rx) = mpsc::channel();
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();
    app.add_plugins(NetQaPlugin::with_channels(rx));
    // Settle into Running (and run one empty router pass).
    app.update();
    (app, tx)
}

/// The test's own ground truth of which kinds need a battle in progress — the
/// battle-only reads/writes/injects, `ScreenshotAfter` among them (GTW-749: it embeds an
/// intent, so it needs exactly what a bare `Inject` needs). Kept independent of the
/// router so a drift between the router and its advertised set is caught rather than
/// trusted away.
const fn is_battle_dependent(kind: RequestKindNet) -> bool {
    matches!(
        kind,
        RequestKindNet::Inject
            | RequestKindNet::GetBattleState
            | RequestKindNet::GetOutput
            | RequestKindNet::ScreenshotAfter
    )
}

/// A representative request of each kind for probing the router's accept/reject. The
/// wildcard-free `match` forces a new kind to grow a probe here.
///
/// `StartBattle` names a deliberately unknown situation so an ACCEPTED (router-level)
/// request is then answered `BadRequest` by the navigation consumer without descending into
/// a battle — the probe tests the ROUTER's accept (not-`NoBattle`), not the navigation.
fn probe_request(kind: RequestKindNet) -> QaRequest {
    match kind {
        RequestKindNet::Hello => QaRequest::Hello(NET_QA_PROTOCOL_VERSION),
        RequestKindNet::GetAppFlow => QaRequest::GetAppFlow,
        RequestKindNet::GetBattleState => QaRequest::GetBattleState,
        RequestKindNet::Inject => QaRequest::Inject(NetIntent::Reload),
        RequestKindNet::TakeScreenshot => QaRequest::TakeScreenshot { name: None },
        RequestKindNet::ScreenshotAfter => QaRequest::ScreenshotAfter {
            intent:      NetIntent::Reload,
            frame_delay: FrameDelay::new(0),
            name:        None,
        },
        RequestKindNet::GetOutput => QaRequest::GetOutput { max: None },
        RequestKindNet::StartBattle => QaRequest::StartBattle {
            situation: SituationRef::new("affordance-probe-unknown".to_owned()),
            seed:      None,
        },
        RequestKindNet::StepperControl => QaRequest::StepperControl(StepperCommandNet::Next),
        // A deliberately bogus token: `ActivateMenuItem` is always serviceable at the
        // router level (like `StartBattle`), so this probes the router's ACCEPT — the
        // consumer then answers the stale token `Rejected(StaleToken)`, not `NoBattle`.
        RequestKindNet::ActivateMenuItem => {
            QaRequest::ActivateMenuItem(gdtf_qa_protocol::ids::FocusTargetNet::new(0))
        }
    }
}

/// Send `GetAppFlow`, drive one frame, and read the affordance list off the reply.
fn advertised_available(app: &mut App, tx: &mpsc::Sender<IncomingRequest>) -> Vec<RequestKindNet> {
    let reply = send(tx, QaRequest::GetAppFlow);
    app.update();
    let Ok(QaResponse::AppFlow(view)) = reply.try_recv() else {
        unreachable!("GetAppFlow must answer with an AppFlow snapshot");
    };
    view.available
}

/// The load-bearing invariant: the advertised affordance list and the router's real
/// accept/reject cannot disagree in this state (see the module doc).
///
/// None of these fixtures has a live procgen-stepper drive, so `StepperControl` (GTW-766) is
/// always unadvertised here and rejected with its OWN reason,
/// [`QaError::StepperInactive`] — never the battle-dependent `NoBattle`.
fn assert_affordance_parity(app: &mut App, tx: &mpsc::Sender<IncomingRequest>) {
    let advertised = advertised_available(app, tx);
    for kind in RequestKindNet::ALL {
        let probe = send(tx, probe_request(kind));
        app.update();
        let reply = probe.try_recv();
        let is_no_battle = matches!(&reply, Ok(QaResponse::Error(QaError::NoBattle)));
        let is_stepper_inactive = matches!(&reply, Ok(QaResponse::Error(QaError::StepperInactive)));
        if advertised.contains(&kind) {
            assert!(
                !is_no_battle && !is_stepper_inactive,
                "{kind:?} was advertised available but the router rejected it unavailable: \
                 {reply:?}",
            );
        } else if kind == RequestKindNet::StepperControl {
            // Unadvertised because no procgen-stepper drive is in flight — rejected with its
            // own accurate reason, not a NoBattle lie.
            assert!(
                is_stepper_inactive,
                "StepperControl was not advertised but the router did not reject it \
                 StepperInactive: {reply:?}",
            );
        } else {
            assert!(
                is_battle_dependent(kind),
                "{kind:?} was not advertised, but only battle- or stepper-dependent kinds may \
                 be absent",
            );
            assert!(
                is_no_battle,
                "{kind:?} was not advertised but the router did not reject it NoBattle: \
                 {reply:?}",
            );
        }
    }
}

/// No battle running: the affordance list omits the battle-only trio, and the router agrees
/// (accepts the four always-serviceable kinds, rejects the trio `NoBattle`).
#[test]
fn affordances_match_router_with_no_battle() {
    let (mut app, tx) = no_battle_app();
    assert_affordance_parity(&mut app, &tx);
}

/// At the main menu (the state a QA client sits in before starting a battle, now that T9
/// landed): the same four kinds are advertised — `StartBattle` among them — and the trio is
/// still rejected `NoBattle`, exactly as the router behaves.
#[test]
fn affordances_match_router_at_the_menu() {
    let (mut app, tx) = menu_app_with_net_qa();
    assert_affordance_parity(&mut app, &tx);
}

/// In a live battle: every request kind is advertised, and the router accepts all of them
/// (the trio is now serviceable), so nothing is rejected `NoBattle`.
#[test]
fn affordances_match_router_in_battle() {
    let Some((mut app, tx)) = inject_battle_app() else {
        return;
    };
    assert_affordance_parity(&mut app, &tx);
}
