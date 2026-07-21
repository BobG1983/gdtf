//! The start-battle navigation consumer (GTW-742, the GTW-694 architecture's T9).
//!
//! [`drive_start_battle`] drains the routed [`StartBattlePayload`] queue the T3 router
//! fills and, for each accepted request, produces the SAME
//! [`StartBattleRequested`](crate::states::running::menu::StartBattleRequested) message
//! the menu's Battlescape button produces — so the network path and the local UI path
//! share one truth. It never writes `NextState` itself: the menu's
//! [`apply_start_battle`](crate::states::running::menu) consumer owns the actual
//! `Menu → Game` transition (and installs the pinned seed override), exactly as the
//! local button path routes through it.
//!
//! A request is accepted only at the main menu with no battle already running, and only
//! when its [`SituationRef`] names a shipped situation; otherwise it is answered with a
//! typed [`QaError::BadRequest`] — never a panic, never a silent no-op.

use bevy::prelude::*;
use gdtf_battle_sim::{prelude::BattleInProgress, rng::BattleSeed};
use gdtf_qa_protocol::{
    envelope::{QaError, QaResponse},
    ids::SituationRef,
    view::{AppFlowView, BattleActiveNet, CaughtUpNet},
};

use super::{
    pending::{PendingQueue, StartBattlePayload},
    router::{app_state_to_net, available_requests},
};
use crate::states::{AppState, RunningState, running::menu::StartBattleRequested};

/// The catch-up fact reported on a `StartBattle` acknowledgement.
///
/// Always `true`: the acknowledgement is sent from the MENU, before the descent into the
/// battle, so there is nothing being replayed and nothing to be behind on. Named rather
/// than written as a bare literal so the reason travels with the value.
const CAUGHT_UP_AT_MENU: bool = true;

/// The one authored situation the game currently ships — the stem of the Load scene's
/// `content/situations/skirmish.ron`. `drive_start_battle` accepts a
/// [`StartBattle`](gdtf_qa_protocol::envelope::QaRequest::StartBattle) naming this
/// situation and rejects any other with [`QaError::BadRequest`].
///
/// A single-entry catalog is the honest current reality (the game ships exactly one
/// authored situation); a real multi-situation catalog — and exposing the valid names
/// over the wire so a QA client can discover them — is future scope, tracked with the
/// broader app-flow navigation follow-up (GTW-747). Widened to `pub` under
/// `test-support` so the T9 integration test names the valid situation without
/// hard-coding the literal.
#[cfg(feature = "test-support")]
pub const SHIPPED_SITUATION: &str = "skirmish";
/// See the `test-support` variant — the shipped-situation name the resolver accepts.
#[cfg(not(feature = "test-support"))]
pub(super) const SHIPPED_SITUATION: &str = "skirmish";

/// Whether a client's [`SituationRef`] names a situation the game can start.
fn situation_is_known(situation: &SituationRef) -> bool {
    situation.as_str() == SHIPPED_SITUATION
}

/// Drain the routed [`StartBattlePayload`] queue and start a battle for each accepted
/// request, answering every request THIS frame (GTW-742).
///
/// Registered in [`InputSystems::Gather`](gdtf_battle_input::InputSystems)
/// `.after(route_requests)` by [`super::plugin`] — so it sees the same frame's routed
/// pushes. Always runs (so it can reject a request that arrives in the wrong state
/// rather than leave it to the deadline sweep). Per request:
///
/// - Not at the main menu, or a battle already running → [`QaError::BadRequest`] (a
///   start-battle navigation is only meaningful from the menu).
/// - An unknown situation ref → [`QaError::BadRequest`].
/// - Otherwise → write the shared
///   [`StartBattleRequested`](crate::states::running::menu::StartBattleRequested)
///   (with the pinned seed, if any) that the menu's `apply_start_battle` applies, and
///   answer with the current app-flow snapshot (the accepted acknowledgement — the
///   client polls `GetAppFlow` / `GetBattleState` to watch the descent to a battle).
pub(super) fn drive_start_battle(
    mut queue: ResMut<PendingQueue<StartBattlePayload>>,
    app_state: Res<State<AppState>>,
    running_state: Option<Res<State<RunningState>>>,
    battle: Option<Res<BattleInProgress>>,
    mut start: MessageWriter<StartBattleRequested>,
) {
    let at_menu = running_state
        .as_deref()
        .map(State::get)
        .is_some_and(|state| matches!(state, RunningState::Menu));
    let in_battle = battle.is_some();
    for (payload, responder) in queue.drain_ready() {
        if in_battle || !at_menu {
            responder.reply(QaResponse::Error(QaError::BadRequest));
            continue;
        }
        if !situation_is_known(payload.situation()) {
            responder.reply(QaResponse::Error(QaError::BadRequest));
            continue;
        }
        // Produce the SAME message the local Battlescape button produces, carrying the
        // wire seed mapped onto the sim's `BattleSeed` override (or `None` — the normal
        // `resolve_root_seed` path). The menu's `apply_start_battle` installs the seed
        // and performs the `Menu → Game` transition.
        let seed = payload.seed().map(|seed| BattleSeed::new(*seed));
        start.write(StartBattleRequested::new(seed));
        // Acknowledge with the current app-flow snapshot (still at the menu — the descent
        // is deferred), reporting the affordances the same predicate the router uses
        // advertises for this state.
        responder.reply(QaResponse::AppFlow(AppFlowView::new(
            app_state_to_net(app_state.get()),
            BattleActiveNet::new(in_battle),
            available_requests(in_battle, CAUGHT_UP_AT_MENU),
            CaughtUpNet::new(CAUGHT_UP_AT_MENU),
        )));
    }
}
