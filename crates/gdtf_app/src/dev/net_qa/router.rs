//! The always-on request router (GTW-736).
//!
//! [`route_requests`] drains the [`NetInbox`] every frame in the
//! [`InputSystems::Gather`](gdtf_battle_input::InputSystems) band and dispatches each
//! [`QaRequest`]:
//!
//! - [`Hello`](QaRequest::Hello) and [`GetAppFlow`](QaRequest::GetAppFlow) are answered
//!   directly (a protocol handshake / a lifecycle read need no battle).
//! - The battle-dependent requests ([`Inject`](QaRequest::Inject) /
//!   [`GetBattleState`](QaRequest::GetBattleState) / [`GetOutput`](QaRequest::GetOutput))
//!   are rejected [`NoBattle`](QaError::NoBattle) at ROUTE time via
//!   `Option<Res<BattleInProgress>>` — never a panic on the missing resource
//!   (bevy-traps #1) — and otherwise enqueued for their (later) consumer.
//! - Everything else ([`TakeScreenshot`](QaRequest::TakeScreenshot) /
//!   [`StartBattle`](QaRequest::StartBattle)) is enqueued unconditionally.
//!
//! The system is ALWAYS registered when the plugin is active; per-request behavior varies
//! but the system itself never blinks in and out.

use bevy::prelude::*;
use gdtf_battle_sim::prelude::BattleInProgress;
use gdtf_qa_protocol::{
    envelope::{HelloFacts, ProtocolVersion, QaError, QaRequest, QaResponse, ServerNameNet},
    view::{AppFlowView, AppStateNet, BattleActiveNet},
};

use super::{
    channel::{NetInbox, Responder},
    config::{NET_QA_PROTOCOL_VERSION, SERVER_NAME},
    pending::{
        InjectPayload, OutputPayload, PendingQueues, ScreenshotPayload, SnapshotPayload,
        StartBattlePayload,
    },
};
use crate::states::AppState;

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
        match request {
            QaRequest::Hello(client_version) => answer_hello(client_version, responder),
            QaRequest::GetAppFlow => {
                let view = AppFlowView::new(
                    app_state_to_net(app_state.get()),
                    BattleActiveNet::new(in_battle),
                );
                responder.reply(QaResponse::AppFlow(view));
            }
            // The battle-dependent trio: rejected NoBattle at route time, else enqueued
            // for the (later) T4/T5/T6 consumer.
            QaRequest::Inject(_) | QaRequest::GetBattleState | QaRequest::GetOutput { .. }
                if !in_battle =>
            {
                reject_no_battle(responder);
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
            // Not battle-dependent: enqueued unconditionally for the (later) T7/T9 consumer.
            QaRequest::TakeScreenshot { name } => {
                queues
                    .screenshot
                    .push_new(ScreenshotPayload::new(name), responder);
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
