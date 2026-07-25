//! The always-on request router (GTW-736; the affordance advertisement is GTW-746).
//!
//! [`route_requests`] drains the [`NetInbox`] every frame in the
//! [`InputSystems::Gather`](gdtf_battle_input::InputSystems) band and dispatches each
//! [`QaRequest`]. One predicate, [`request_available`], decides which requests the router
//! services in the current state:
//!
//! - A request the router cannot service now is rejected at ROUTE time via
//!   `Option<Res<_>>` witnesses — never a panic on the missing resource (bevy-traps #1). The
//!   battle-dependent quartet ([`Inject`](QaRequest::Inject) /
//!   [`GetBattleState`](QaRequest::GetBattleState) / [`GetOutput`](QaRequest::GetOutput) /
//!   [`ScreenshotAfter`](QaRequest::ScreenshotAfter)) arriving with no battle running is
//!   rejected [`NoBattle`](QaError::NoBattle) (`ScreenshotAfter` embeds an intent, so it is
//!   gated exactly like a bare `Inject`, GTW-749 — never a stale capture); the DEV
//!   [`StepperControl`](QaRequest::StepperControl) arriving with no live procgen-stepper
//!   drive is rejected [`StepperInactive`](QaError::StepperInactive) (GTW-766).
//! - A serviceable request is then answered directly ([`Hello`](QaRequest::Hello) /
//!   [`GetAppFlow`](QaRequest::GetAppFlow)) or enqueued for its (later) consumer
//!   ([`Inject`](QaRequest::Inject) / [`GetBattleState`](QaRequest::GetBattleState) /
//!   [`GetOutput`](QaRequest::GetOutput) / [`TakeScreenshot`](QaRequest::TakeScreenshot) /
//!   [`ScreenshotAfter`](QaRequest::ScreenshotAfter) /
//!   [`StartBattle`](QaRequest::StartBattle) /
//!   [`StepperControl`](QaRequest::StepperControl)).
//!
//! The `GetAppFlow` answer reports the same set — [`available_requests`] filters
//! [`RequestKindNet::ALL`] through the SAME [`request_available`] predicate — so what a QA
//! client is told it may send and what the router actually accepts are computed from one
//! source and cannot disagree.
//!
//! The system is ALWAYS registered when the plugin is active; per-request behavior varies
//! but the system itself never blinks in and out.

use bevy::prelude::*;
use gdtf_battle_presenter::PlaybackGate;
use gdtf_battle_sim::{prelude::BattleInProgress, procgen::StagedProcgen};
use gdtf_net_qa_transport::{NetInbox, Responder};
use gdtf_qa_protocol::{
    envelope::{HelloFacts, ProtocolVersion, QaError, QaRequest, QaResponse, ServerNameNet},
    view::{AppFlowView, AppStateNet, BattleActiveNet, CaughtUpNet, RequestKindNet},
};

use super::{
    config::{NET_QA_PROTOCOL_VERSION, SERVER_NAME},
    pending::{
        ActivateMenuPayload, FocusControlPayload, InjectPayload, OutputPayload, PendingQueues,
        ScreenshotAfterPayload, ScreenshotPayload, SnapshotPayload, StartBattlePayload,
        StepperControlPayload,
    },
    snapshot::AppFlowViews,
};
use crate::states::AppState;

crate::support_item! {
    /// Whether the router will service `kind` given the state facts it keys accept/reject on.
    ///
    /// The single predicate that governs BOTH the route-time accept/reject below and the
    /// advertised `available_requests` set, so the two can never drift. The battle-dependent
    /// quartet needs a battle in progress; the DEV
    /// [`StepperControl`](RequestKindNet::StepperControl) needs a live procgen-stepper drive;
    /// every other request kind is serviceable regardless of state. The app state is read only
    /// to fill the snapshot, not to gate requests.
    ///
    /// [`ScreenshotAfter`](RequestKindNet::ScreenshotAfter) joins the battle-dependent group
    /// (GTW-749): it embeds a battle intent, so it needs exactly what a bare
    /// [`Inject`](RequestKindNet::Inject) needs — a live battle to inject into.
    ///
    /// `stepper_active` (GTW-766) is `true` only while a battle's procgen `Generation` is
    /// being driven a stage at a time (a live `StagedProcgen`), NOT merely because the process
    /// has the DEV stepper feature on — see `route_requests` for where it is read.
    ///
    /// Widened to `pub` under `test-support` (the GTW-727 input-gate suite asserts the
    /// catch-up gating against this exact function) and `pub(crate)` otherwise, so the
    /// binary — which never names it from outside this module — stays `unreachable_pub`-clean.
    #[must_use]
    const fn request_available(
        kind: RequestKindNet,
        in_battle: bool,
        caught_up: bool,
        stepper_active: bool,
    ) -> bool {
        match kind {
            // ACT-BEARING (GTW-727 C42): these carry a battle intent, so they need what the
            // player needs — a live battle AND a screen that is current. Injecting an act while
            // an exchange is still being replayed is the QA-side of exactly the defect this
            // pacing exists to fix: acting on information the screen has not shown.
            RequestKindNet::Inject | RequestKindNet::ScreenshotAfter => in_battle && caught_up,
            // READS: a live battle is enough. `GetOutput` in particular is NEVER gated on
            // catch-up (C44) — it is the client's observation channel, and throttling it while
            // pacing happens would make an agent unable to watch the very thing it is testing.
            RequestKindNet::GetBattleState | RequestKindNet::GetOutput => in_battle,
            // The DEV stepper-control request (GTW-766) needs the procgen stepper to be
            // actively driving RIGHT NOW — a live `StagedProcgen` — not just a battle.
            RequestKindNet::StepperControl => stepper_active,
            // A menu-item activation (GTW-787) is always serviceable at the router level,
            // exactly like `StartBattle`: whether the token names a live, listed menu item
            // is the consumer's check (a stale one is answered `Rejected(StaleToken)`), not
            // a route-time state gate. A focus drive (GTW-802) joins that group for the same
            // reason — and deliberately is NOT battle-gated: the screens it drives (Options,
            // the menu) are off-battle, which is exactly the gap it closes.
            RequestKindNet::Hello
            | RequestKindNet::GetAppFlow
            | RequestKindNet::TakeScreenshot
            | RequestKindNet::StartBattle
            | RequestKindNet::ActivateMenuItem
            | RequestKindNet::FocusControl => true,
            // The CONTENT EDITOR's query pair (GTW-805, ADR 0007). One request enum serves
            // both hosts, and this host is the GAME: it runs no editor and holds none of the
            // authoring model, so these are never serviceable here, in any state. They are
            // therefore never advertised, and a client that sends one anyway is answered
            // `BadRequest` (see `reject_unavailable`) — the editor's own host on its own port
            // is where they are answered.
            RequestKindNet::GetEditorQueryOptions | RequestKindNet::QueryEditor => false,
        }
    }
}

/// The request kinds the server will service right now — [`RequestKindNet::ALL`] filtered
/// through [`request_available`]. This is exactly what [`GetAppFlow`](QaRequest::GetAppFlow)
/// advertises, and what an accepted [`StartBattle`](QaRequest::StartBattle) acknowledgement
/// reports (see [`drive_start_battle`](super::start_battle::drive_start_battle)).
pub(super) fn available_requests(
    in_battle: bool,
    caught_up: bool,
    stepper_active: bool,
) -> Vec<RequestKindNet> {
    RequestKindNet::ALL
        .into_iter()
        .filter(|kind| request_available(*kind, in_battle, caught_up, stepper_active))
        .collect()
}

/// Drains the inbox and dispatches every buffered request (see the module doc).
pub(super) fn route_requests(
    inbox: Res<NetInbox>,
    app_state: Res<State<AppState>>,
    battle: Option<Res<BattleInProgress>>,
    staged: Option<Res<StagedProcgen>>,
    playback: PlaybackGate,
    views: AppFlowViews,
    mut queues: PendingQueues,
) {
    let in_battle = battle.is_some();
    // A live drive is in flight iff the sim's `StagedProcgen` resource exists — the per-span
    // "the stepper is driving RIGHT NOW" signal, inserted only by the DEV stepper's
    // `engage_stepper`. Without the `dev_tools` stepper nothing ever inserts it, so this is
    // always `false` and every `StepperControl` is rejected `StepperInactive`.
    let stepper_active = staged.is_some();
    let caught_up = playback.is_open();
    for incoming in inbox.drain() {
        let (request, responder) = incoming.into_parts();
        // A request the router cannot service in this state is rejected AT ROUTE TIME, not
        // queued: the pending-queue deadline is a few frames, far shorter than a catch-up
        // window, so holding a gated inject would time out virtually every one of them and
        // force a deadline retune for every other request kind too.
        //
        // Rejecting here also preserves the honest-receipt contract: a client is never told
        // `Queued` for an act the drain will silently discard.
        if !request_available(request.kind(), in_battle, caught_up, stepper_active) {
            reject_unavailable(request.kind(), in_battle, responder);
            continue;
        }
        match request {
            QaRequest::Hello(client_version) => answer_hello(client_version, responder),
            QaRequest::GetAppFlow => {
                let view = AppFlowView::new(
                    app_state_to_net(app_state.get()),
                    BattleActiveNet::new(in_battle),
                    available_requests(in_battle, caught_up, stepper_active),
                    CaughtUpNet::new(caught_up),
                    views.menu(),
                    views.focus(),
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
            QaRequest::StepperControl(command) => {
                queues
                    .stepper_control
                    .push_new(StepperControlPayload::new(command), responder);
            }
            QaRequest::ActivateMenuItem(token) => {
                queues
                    .activate_menu
                    .push_new(ActivateMenuPayload::new(token), responder);
            }
            QaRequest::FocusControl(command) => {
                queues
                    .focus_control
                    .push_new(FocusControlPayload::new(command), responder);
            }
            // Unreachable in practice — `request_available` refuses the editor kinds above,
            // so they are answered `BadRequest` before this match. Listed exhaustively (no
            // wildcard arm) so a future request variant fails to compile here rather than
            // being silently swallowed by a catch-all.
            QaRequest::GetEditorQueryOptions | QaRequest::QueryEditor(_) => {
                responder.reply(QaResponse::Error(QaError::BadRequest));
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

/// Reject an unavailable request with an ACCURATE reason (GTW-727 C43, GTW-766).
///
/// Answering the wrong reason is a lie a client cannot recover from: `NoBattle` during a live
/// battle would send an agent off to start a battle that is already running. So the reason is
/// derived from the same facts the availability predicate used. There are three reasons a
/// request can be refused at route time:
///
/// - [`StepperControl`](RequestKindNet::StepperControl) is gated ONLY on a live stepper drive,
///   so reaching here means there is none — [`StepperInactive`](QaError::StepperInactive),
///   never a `NoBattle` lie (a battle may well be running);
/// - the CONTENT EDITOR's query kinds (GTW-805) are not a state gate at all: this host has
///   no editor to read, in any state — [`BadRequest`](QaError::BadRequest), the same answer
///   the editor's host gives a game-only request;
/// - anything else with no battle is [`NoBattle`](QaError::NoBattle);
/// - anything else with a battle can only be the catch-up gate —
///   [`NotCaughtUp`](QaError::NotCaughtUp).
fn reject_unavailable(kind: RequestKindNet, in_battle: bool, responder: Responder) {
    let error = if matches!(kind, RequestKindNet::StepperControl) {
        QaError::StepperInactive
    } else if matches!(
        kind,
        RequestKindNet::GetEditorQueryOptions | RequestKindNet::QueryEditor
    ) {
        QaError::BadRequest
    } else if in_battle {
        QaError::NotCaughtUp
    } else {
        QaError::NoBattle
    };
    responder.reply(QaResponse::Error(error));
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
