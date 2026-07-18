//! The always-on request router (GTW-736; the affordance advertisement is GTW-746).
//!
//! [`route_requests`] drains the [`NetInbox`] every frame in the
//! [`InputSystems::Gather`](gdtf_battle_input::InputSystems) band and dispatches each
//! [`QaRequest`]. One predicate, [`request_available`], decides which requests the router
//! services in the current state:
//!
//! - A request the router cannot service now is rejected [`NoBattle`](QaError::NoBattle)
//!   at ROUTE time via `Option<Res<BattleInProgress>>` — never a panic on the missing
//!   resource (bevy-traps #1). Today the reason a request is unserviceable is the
//!   battle-dependent quartet ([`Inject`](QaRequest::Inject) /
//!   [`GetBattleState`](QaRequest::GetBattleState) / [`GetOutput`](QaRequest::GetOutput) /
//!   [`ScreenshotAfter`](QaRequest::ScreenshotAfter)) arriving with no battle running —
//!   `ScreenshotAfter` embeds an intent, so it is gated exactly like a bare `Inject`
//!   (GTW-749): a rejection there is the SAME route-time `NoBattle`, never a stale
//!   capture.
//! - A serviceable request is then answered directly ([`Hello`](QaRequest::Hello) /
//!   [`GetAppFlow`](QaRequest::GetAppFlow)) or enqueued for its (later) consumer
//!   ([`Inject`](QaRequest::Inject) / [`GetBattleState`](QaRequest::GetBattleState) /
//!   [`GetOutput`](QaRequest::GetOutput) / [`TakeScreenshot`](QaRequest::TakeScreenshot) /
//!   [`ScreenshotAfter`](QaRequest::ScreenshotAfter) /
//!   [`StartBattle`](QaRequest::StartBattle)).
//!
//! The `GetAppFlow` answer reports the same set — [`available_requests`] filters
//! [`RequestKindNet::ALL`] through the SAME [`request_available`] predicate — so what a QA
//! client is told it may send and what the router actually accepts are computed from one
//! source and cannot disagree.
//!
//! The system is ALWAYS registered when the plugin is active; per-request behavior varies
//! but the system itself never blinks in and out.

use bevy::prelude::*;
use gdtf_battle_sim::prelude::BattleInProgress;
use gdtf_qa_protocol::{
    envelope::{HelloFacts, ProtocolVersion, QaError, QaRequest, QaResponse, ServerNameNet},
    view::{AppFlowView, AppStateNet, BattleActiveNet, RequestKindNet},
};

use super::{
    channel::{NetInbox, Responder},
    config::{NET_QA_PROTOCOL_VERSION, SERVER_NAME},
    pending::{
        InjectPayload, OutputPayload, PendingQueues, ScreenshotAfterPayload, ScreenshotPayload,
        SnapshotPayload, StartBattlePayload,
    },
};
use crate::states::AppState;

/// Whether the router will service `kind` given the state facts it keys accept/reject on.
///
/// The single predicate that governs BOTH the route-time accept/reject below and the
/// advertised [`available_requests`] set, so the two can never drift. The battle-dependent
/// quartet needs a battle in progress; every other request kind is serviceable regardless
/// of state. `in_battle` is the only fact accept/reject keys on today — the app state is
/// read only to fill the snapshot, not to gate requests.
///
/// [`ScreenshotAfter`](RequestKindNet::ScreenshotAfter) joins the battle-dependent group
/// (GTW-749): it embeds a battle intent, so it needs exactly what a bare
/// [`Inject`](RequestKindNet::Inject) needs — a live battle to inject into.
const fn request_available(kind: RequestKindNet, in_battle: bool) -> bool {
    match kind {
        RequestKindNet::Inject
        | RequestKindNet::GetBattleState
        | RequestKindNet::GetOutput
        | RequestKindNet::ScreenshotAfter => in_battle,
        RequestKindNet::Hello
        | RequestKindNet::GetAppFlow
        | RequestKindNet::TakeScreenshot
        | RequestKindNet::StartBattle => true,
    }
}

/// The request kinds the server will service right now — [`RequestKindNet::ALL`] filtered
/// through [`request_available`]. This is exactly what [`GetAppFlow`](QaRequest::GetAppFlow)
/// advertises, and what an accepted [`StartBattle`](QaRequest::StartBattle) acknowledgement
/// reports (see [`drive_start_battle`](super::start_battle::drive_start_battle)).
pub(super) fn available_requests(in_battle: bool) -> Vec<RequestKindNet> {
    RequestKindNet::ALL
        .into_iter()
        .filter(|kind| request_available(*kind, in_battle))
        .collect()
}

/// Drains the inbox and dispatches every buffered request (see the module doc).
pub(super) fn route_requests(
    inbox: Res<NetInbox>,
    app_state: Res<State<AppState>>,
    battle: Option<Res<BattleInProgress>>,
    mut queues: PendingQueues,
) {
    let in_battle = battle.is_some();
    for incoming in inbox.drain() {
        let (request, responder) = incoming.into_parts();
        // A request the router cannot service in this state is rejected at route time. The
        // only current unavailability reason is a battle-dependent request with no battle
        // running, answered NoBattle — reading `battle` as an `Option` so the missing
        // resource is never a panic (bevy-traps #1).
        if !request_available(request.kind(), in_battle) {
            reject_no_battle(responder);
            continue;
        }
        match request {
            QaRequest::Hello(client_version) => answer_hello(client_version, responder),
            QaRequest::GetAppFlow => {
                let view = AppFlowView::new(
                    app_state_to_net(app_state.get()),
                    BattleActiveNet::new(in_battle),
                    available_requests(in_battle),
                );
                responder.reply(QaResponse::AppFlow(view));
            }
            QaRequest::Inject(intent) => {
                queues
                    .inject
                    .push_new(InjectPayload::new(intent), responder);
            }
            QaRequest::GetBattleState => queues.snapshot.push_new(SnapshotPayload, responder),
            QaRequest::GetOutput { max } => {
                queues.output.push_new(OutputPayload::new(max), responder);
            }
            QaRequest::TakeScreenshot { name } => {
                queues
                    .screenshot
                    .push_new(ScreenshotPayload::new(name), responder);
            }
            QaRequest::ScreenshotAfter {
                intent,
                frame_delay,
                name,
            } => {
                queues.screenshot_after.push_new(
                    ScreenshotAfterPayload::new(intent, frame_delay, name),
                    responder,
                );
            }
            QaRequest::StartBattle { situation, seed } => {
                queues
                    .start_battle
                    .push_new(StartBattlePayload::new(situation, seed), responder);
            }
        }
    }
}

/// Negotiate the protocol version: reply the handshake facts on a match, or
/// [`VersionMismatch`](QaError::VersionMismatch) otherwise.
fn answer_hello(client_version: ProtocolVersion, responder: Responder) {
    if client_version == NET_QA_PROTOCOL_VERSION {
        let facts = HelloFacts::new(
            NET_QA_PROTOCOL_VERSION,
            ServerNameNet::new(SERVER_NAME.to_owned()),
        );
        responder.reply(QaResponse::HelloOk(facts));
    } else {
        responder.reply(QaResponse::Error(QaError::VersionMismatch));
    }
}

/// Reject a battle-dependent request that arrived with no battle in progress.
fn reject_no_battle(responder: Responder) {
    responder.reply(QaResponse::Error(QaError::NoBattle));
}

/// Map the game's top-level [`AppState`] onto its wire mirror. Shared with the T9
/// [`drive_start_battle`](super::start_battle::drive_start_battle) consumer, which
/// answers an accepted `StartBattle` with the same app-flow snapshot.
pub(super) const fn app_state_to_net(state: &AppState) -> AppStateNet {
    match state {
        AppState::Init => AppStateNet::Init,
        AppState::Load => AppStateNet::Load,
        AppState::Intro => AppStateNet::Intro,
        AppState::Running => AppStateNet::Running,
        AppState::Teardown => AppStateNet::Teardown,
    }
}
