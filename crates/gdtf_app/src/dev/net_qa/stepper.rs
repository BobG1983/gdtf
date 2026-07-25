//! The DEV procgen stepper-drive dispatch (GTW-766).
//!
//! [`drive_stepper_control`] drains the routed [`StepperControlPayload`] queue the router
//! fills and writes each command into the SAME idempotent latch the egui panel's Next / Auto
//! / Skip buttons write (the game-side `PendingStepCommand` / `AutoRunning`) — never a new
//! queue, never a second side-channel. The drive system (`advance_stepper_drive`) drains that
//! latch once per frame, exactly as it does for a local button press, so two commands in one
//! frame both assign the SAME single-slot latch and apply once (the panel's own idempotence,
//! preserved — bevy-traps #8b).
//!
//! The router already gates a `StepperControl` on a live `StagedProcgen`
//! (route-time [`StepperInactive`](gdtf_qa_protocol::envelope::QaError::StepperInactive)), so
//! the stepper's latch resources exist by the time a command reaches here; the fail-closed
//! [`Inactive`](gdtf_qa_protocol::envelope::StepperReceipt::Inactive) reply is
//! defense-in-depth (bevy-traps #1: never panic on an absent resource).
//!
//! Compiled in TWO forms so `net_qa` alone still builds: with `dev_tools` it reaches into the
//! stepper's latch resources; without it (the stepper module does not exist at all) it is a
//! drain-and-answer stub — in that build a `StagedProcgen` is never inserted, so the router
//! already route-rejects every `StepperControl`, and this body only answers a request that
//! cannot arrive.

use bevy::prelude::*;
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_protocol::envelope::{QaResponse, StepperReceipt};

use super::pending::StepperControlPayload;
#[cfg(feature = "dev_tools")]
use crate::dev::procgen_stepper::{AutoRunning, PendingStepCommand, StepCommand};

/// Drain the routed [`StepperControlPayload`] queue and write each command into the stepper's
/// latch, answering every request THIS frame with its
/// [`StepperReceipt`](gdtf_qa_protocol::envelope::StepperReceipt) (GTW-766).
///
/// Registered UNCONDITIONALLY in [`InputSystems::Gather`](gdtf_battle_input::InputSystems)
/// `.after(route_requests)` by [`super::plugin`] — so it drains the request the router just
/// routed the same frame. This is the `dev_tools` body: it maps the wire command onto the
/// game's own `PendingStepCommand` / `AutoRunning` latch, exactly as an egui button press
/// does. A ±1-frame lag between this dispatch and the once-per-frame `advance_stepper_drive`
/// that consumes the latch is acceptable: the latch is a purpose-built idempotent
/// drained-once slot (the same property the egui panel relies on across the
/// `EguiPrimaryContextPass` → `Update` boundary).
#[cfg(feature = "dev_tools")]
pub(super) fn drive_stepper_control(
    mut queue: ResMut<PendingQueue<StepperControlPayload>>,
    mut pending: Option<ResMut<PendingStepCommand>>,
    mut auto: Option<ResMut<AutoRunning>>,
) {
    for (payload, responder) in queue.drain_ready() {
        let receipt = apply_command(
            payload.command(),
            pending.as_deref_mut(),
            auto.as_deref_mut(),
        );
        responder.reply(QaResponse::StepperControlled(receipt));
    }
}

/// Map one wire stepper command onto the game's latch, reporting whether it landed.
///
/// A missing latch resource (the stepper is not engaged) fails closed to
/// [`Inactive`](StepperReceipt::Inactive) rather than panicking — unreachable in practice
/// (the router gates on the live drive), but defense-in-depth (bevy-traps #1).
#[cfg(feature = "dev_tools")]
fn apply_command(
    command: gdtf_qa_protocol::envelope::StepperCommandNet,
    pending: Option<&mut PendingStepCommand>,
    auto: Option<&mut AutoRunning>,
) -> StepperReceipt {
    use gdtf_qa_protocol::envelope::StepperCommandNet;
    match command {
        StepperCommandNet::Next => latch_step(pending, StepCommand::Next),
        StepperCommandNet::Skip => latch_step(pending, StepCommand::Skip),
        StepperCommandNet::Auto { running } => {
            let Some(auto) = auto else {
                return StepperReceipt::Inactive;
            };
            // Absolute set (never a flip) — the wire mirror of the panel's Start/Stop Auto
            // buttons; `*running` derefs the `AutoRunNet` newtype to its `bool`.
            auto.set(*running);
            StepperReceipt::Latched
        }
    }
}

/// Latch a step command (Next / Skip) into `PendingStepCommand`, or report
/// [`Inactive`](StepperReceipt::Inactive) if the stepper is not engaged.
#[cfg(feature = "dev_tools")]
const fn latch_step(
    pending: Option<&mut PendingStepCommand>,
    command: StepCommand,
) -> StepperReceipt {
    let Some(pending) = pending else {
        return StepperReceipt::Inactive;
    };
    pending.request(command);
    StepperReceipt::Latched
}

/// The `net_qa`-only build's drain-and-answer stub (GTW-766): with the DEV stepper module
/// absent, no `StagedProcgen` is ever inserted, so the router route-rejects every
/// `StepperControl` [`StepperInactive`](gdtf_qa_protocol::envelope::QaError::StepperInactive)
/// before it is queued. This only answers a request that cannot arrive — replying the
/// fail-closed [`Inactive`](gdtf_qa_protocol::envelope::StepperReceipt::Inactive) rather than
/// leaving a (never-created) entry to the deadline sweep. Touches NO stepper type, so the
/// `net_qa`-without-`dev_tools` build compiles.
#[cfg(not(feature = "dev_tools"))]
pub(super) fn drive_stepper_control(mut queue: ResMut<PendingQueue<StepperControlPayload>>) {
    for (_, responder) in queue.drain_ready() {
        responder.reply(QaResponse::StepperControlled(StepperReceipt::Inactive));
    }
}
