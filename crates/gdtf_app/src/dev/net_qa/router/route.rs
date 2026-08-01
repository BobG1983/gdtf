//! [`route_requests`] — the ONE drain of the [`NetInbox`] (GTW-736, GTW-942).

use bevy::prelude::*;
use gdtf_battle_presenter::PlaybackGate;
use gdtf_battle_sim::{prelude::BattleInProgress, procgen::StagedProcgen};
use gdtf_net_qa_transport::NetInbox;
use gdtf_qa_command::{
    catalogue::catalogue,
    dispatch::{Admission, CommandInbox, admit, unavailable_reply, unknown_reply},
};
use gdtf_qa_protocol::{
    envelope::{QaError, QaRequest, QaResponse},
    view::{AppFlowView, AppStateNet, BattleActiveNet, CaughtUpNet},
};

use super::available::{available_requests, reject_unavailable, request_available};
use crate::{
    dev::net_qa::{
        commands::{GAME_COMMANDS, game_host_name},
        facts::GameFactsParam,
        pending::{
            ActivateMenuPayload, FocusControlPayload, InjectPayload, OutputPayload, PendingQueues,
            ScreenshotAfterPayload, ScreenshotPayload, SnapshotPayload, StartBattlePayload,
            StepperControlPayload,
        },
        snapshot::AppFlowViews,
    },
    states::AppState,
};

/// Drains the inbox and dispatches every buffered request (see the module doc).
///
/// The command facts are sampled ONCE, before the drain, not per request: two calls in one
/// frame that disagreed about the world would be answered from two worlds that never
/// existed together.
#[expect(
    clippy::too_many_arguments,
    reason = "every parameter is a distinct resource this one drain needs: the inbox it \
              empties, the state facts three request kinds gate on, the command inbox the \
              Run arm fills, the command facts the Catalogue and Run arms read, and the \
              typed queue bundle every other arm pushes into. Splitting it would mean a \
              SECOND system reading Res<NetInbox>, which is the one thing this file must \
              not have."
)]
pub(in crate::dev::net_qa) fn route_requests(
    inbox: Res<NetInbox>,
    app_state: Res<State<AppState>>,
    battle: Option<Res<BattleInProgress>>,
    staged: Option<Res<StagedProcgen>>,
    playback: PlaybackGate,
    views: AppFlowViews,
    facts: GameFactsParam,
    mut commands: ResMut<CommandInbox>,
    mut queues: PendingQueues,
) {
    let in_battle = battle.is_some();
    // A live drive is in flight iff the sim's `StagedProcgen` resource exists — the per-span
    // "the stepper is driving RIGHT NOW" signal, inserted only by the DEV stepper's
    // `engage_stepper`. Without the `dev_tools` stepper nothing ever inserts it, so this is
    // always `false` and every `StepperControl` is rejected `StepperInactive`.
    let stepper_active = staged.is_some();
    let caught_up = playback.is_open();
    let command_facts = facts.sample();
    for incoming in inbox.drain() {
        let (request, responder) = incoming.into_parts();
        // A request the router cannot service in this state is rejected AT ROUTE TIME, not
        // queued: the pending-queue deadline is a few frames, far shorter than a catch-up
        // window, so holding a gated inject would time out virtually every one of them and
        // force a deadline retune for every other request kind too.
        //
        // Rejecting here also preserves the receipt contract: a client is never told
        // `Queued` for an act the drain will silently discard.
        if !request_available(request.kind(), in_battle, caught_up, stepper_active) {
            reject_unavailable(request.kind(), in_battle, responder);
            continue;
        }
        match request {
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
            // GTW-942 — the command layer's two arms. `Catalogue` is answered from the frame's
            // facts right here, because a catalogue IS the facts read through every command's
            // own predicate and there is nothing to defer.
            QaRequest::Catalogue => {
                responder.reply(QaResponse::Catalogue(catalogue(
                    game_host_name(),
                    GAME_COMMANDS,
                    &command_facts,
                )));
            }
            // A `Run` is RESOLVED here and handled elsewhere: `admit` scans this host's own
            // slice, so an unknown name and an unavailable command are both answered at route
            // time with the correction a caller needs, and only an admitted call is parked for
            // its command's decode step. The name never selects a function pointer — there is
            // no `match` on it anywhere.
            QaRequest::Run(run) => {
                match admit(GAME_COMMANDS, &run.command, &run.options, &command_facts) {
                    Admission::Admit(command) => {
                        commands.admit(command.name(), run.arguments, responder);
                    }
                    Admission::Unavailable(refusal) => responder.reply(unavailable_reply(refusal)),
                    Admission::Unknown(known) => responder.reply(unknown_reply(known)),
                }
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
            // Unreachable in practice: the LISTENER THREAD negotiates a `Hello` against the
            // host's `hello_facts()` and never forwards it (GTW-940), which is why no
            // `answer_hello` lives here any more. One that arrived anyway got past the only
            // code that answers a handshake, so the frame is wrong for this connection —
            // `Malformed`, not the `BadRequest` this host uses for a request it services on
            // other terms.
            QaRequest::Hello(_) => {
                responder.reply(QaResponse::Error(QaError::Malformed));
            }
            // Also unreachable, for a different reason: `request_available` refuses the editor
            // kinds above, so they are answered `BadRequest` before this match. Listed
            // exhaustively (no wildcard arm) so a future request variant fails to compile here
            // rather than being silently swallowed by a catch-all.
            QaRequest::GetEditorQueryOptions | QaRequest::QueryEditor(_) => {
                responder.reply(QaResponse::Error(QaError::BadRequest));
            }
        }
    }
}

/// Map the game's top-level [`AppState`] onto its wire mirror. Shared with the T9
/// [`drive_start_battle`](crate::dev::net_qa::start_battle::drive_start_battle) consumer,
/// which answers an accepted `StartBattle` with the same app-flow snapshot.
pub(in crate::dev::net_qa) const fn app_state_to_net(state: &AppState) -> AppStateNet {
    match state {
        AppState::Init => AppStateNet::Init,
        AppState::Load => AppStateNet::Load,
        AppState::Intro => AppStateNet::Intro,
        AppState::Running => AppStateNet::Running,
        AppState::Teardown => AppStateNet::Teardown,
    }
}
