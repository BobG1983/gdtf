//! Which requests this host services right now, and the accurate reason for a refusal
//! (GTW-746, GTW-727, GTW-766).

use gdtf_net_qa_transport::Responder;
use gdtf_qa_protocol::{
    envelope::{QaError, QaResponse},
    view::RequestKindNet,
};

crate::support_item! {
    /// Whether the router will service `kind` given the state facts it keys accept/reject on.
    ///
    /// The single predicate that governs BOTH the route-time accept/reject in
    /// `route_requests` and the advertised `available_requests` set, so the two can never
    /// drift. The battle-dependent
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
            //
            // The GTW-939 command layer joins them too, since GTW-942 gave this host a command
            // set: `Catalogue` reads a list that exists in every state, and a `Run`'s own
            // preconditions belong to the COMMAND — its availability predicate, answered
            // `Unavailable` with the precondition named — not to a route-time state gate that
            // could only ever answer a blunter error.
            RequestKindNet::Hello
            | RequestKindNet::GetAppFlow
            | RequestKindNet::TakeScreenshot
            | RequestKindNet::StartBattle
            | RequestKindNet::ActivateMenuItem
            | RequestKindNet::FocusControl
            | RequestKindNet::Catalogue
            | RequestKindNet::Run => true,
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
/// through [`request_available`]. This is exactly what
/// [`GetAppFlow`](gdtf_qa_protocol::envelope::QaRequest::GetAppFlow) advertises, and what an
/// accepted [`StartBattle`](gdtf_qa_protocol::envelope::QaRequest::StartBattle)
/// acknowledgement reports (see
/// [`drive_start_battle`](crate::dev::net_qa::start_battle::drive_start_battle)).
pub(in crate::dev::net_qa) fn available_requests(
    in_battle: bool,
    caught_up: bool,
    stepper_active: bool,
) -> Vec<RequestKindNet> {
    RequestKindNet::ALL
        .into_iter()
        .filter(|kind| request_available(*kind, in_battle, caught_up, stepper_active))
        .collect()
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
pub(super) fn reject_unavailable(kind: RequestKindNet, in_battle: bool, responder: Responder) {
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
