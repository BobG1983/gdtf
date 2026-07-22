//! The intent-inject pump (GTW-737, the GTW-694 architecture's T4).
//!
//! [`apply_injects`] drains the routed [`InjectPayload`] queue the T3 router fills and, per
//! intent, pushes it through the SAME public input queues the local surfaces use —
//! [`PendingActIntent`] for the classic acts, the per-act `PendingContextualIntents<A>` for
//! the contextual ones (the SAME write-points the local keyboard / button / panel surfaces
//! use, never a raw `*Requested`) — then answers each request with an
//! [`InjectReceipt`]: `Queued` the moment it enters the queue, or a typed wire-layer
//! rejection. The receipt says
//! nothing about the act's eventual outcome (that is observed downstream via the T5/T6
//! snapshots/outbox) — it is produced HERE, from the wire-layer classification alone.
//!
//! The exhaustive classification lives in [`convert`](super::convert); the world-touching
//! resolution (token liveness, the fire-mode lookup, the offer gate) in
//! [`resolve`](super::resolve). This file owns only the drain + the per-outcome dispatch.

use bevy::prelude::*;
use gdtf_battle_input::{
    ActIntent,
    contextual::{
        EnterEmplacementAct, ExecuteAct, ExitEmplacementAct, MeleeAct, OpenDoorAct, ShoveAct,
        StabilizeAct, ThrowGrenadeAct,
    },
};
use gdtf_battle_sim::acts::{FireRequested, MoveRequested, SetFacingRequested};
use gdtf_qa_protocol::{
    envelope::{InjectReceipt, QaResponse},
    intent::NetIntent,
};

use super::{
    convert::{ActorIntent, Classified, ContextualIntent, cell_level, classify, direction},
    pending::{InjectPayload, PendingQueue},
    resolve::{
        InjectActors, InjectQueues, gate_and_push, resolve_door, resolve_emplacement,
        resolve_fire_mode, resolve_ganger, resolve_melee,
    },
};

/// Drain the routed [`InjectPayload`] queue and process every injected intent through the
/// same public input queues the local surfaces use, answering each with its receipt THIS
/// frame (GTW-737).
///
/// Registered in [`InputSystems::Gather`](gdtf_battle_input::InputSystems)
/// `.after(route_requests)` — so it sees the same frame's routed pushes — and
/// `.before(ContextualActSystems::Drain)` + `.before(dispatch_act_intents)` — so an
/// intent pushed here is drained (and its `*Requested` sim-consumed) the SAME frame (the
/// project's co-schedule same-frame guarantee, the shape the sim's `acts` co-schedule
/// test proves). Gated on a live battle by the plugin. Every reply is emitted from the
/// wire-layer outcome alone (clause 5): it never inspects a downstream act result.
pub(super) fn apply_injects(
    mut injects: ResMut<PendingQueue<InjectPayload>>,
    mut queues: InjectQueues,
    actors: InjectActors,
) {
    for (payload, responder) in injects.drain_ready() {
        let receipt = receipt_for(payload.intent(), &mut queues, &actors);
        responder.reply(QaResponse::Injected(receipt));
    }
}

/// Classify one injected intent and push it onto its input queue, returning its
/// receipt — `Queued` once it enters the queue, or the typed wire-layer rejection.
///
/// `pub(super)` (not private): the T15 `screenshot_after` child (GTW-749) reuses this
/// EXACT classification, so a `ScreenshotAfter`'s embedded intent goes through the same
/// receipt gates a bare [`Inject`](gdtf_qa_protocol::envelope::QaRequest::Inject) does,
/// never a shadow copy.
pub(super) fn receipt_for(
    intent: NetIntent,
    queues: &mut InjectQueues,
    actors: &InjectActors,
) -> InjectReceipt {
    match classify(intent) {
        Classified::Classic(act) => {
            queues.pending.push(act);
            InjectReceipt::Queued
        }
        Classified::Actor(actor_intent) => push_actor(actor_intent, queues, actors),
        Classified::Select(token) => match resolve_ganger(token, actors) {
            Ok(entity) => {
                queues.pending.push(ActIntent::Select(entity));
                InjectReceipt::Queued
            }
            Err(reason) => InjectReceipt::Rejected(reason),
        },
        Classified::Contextual(contextual) => push_contextual(contextual, queues, actors),
    }
}

/// Push an actor-bearing classic intent for the current selection.
///
/// The actor is implicit — the game's `SelectedShooter`. With no selection the intent is
/// a no-op (nothing to act on) but STILL `Queued`: whether an actor exists is an outcome
/// concern, not a wire rejection (clause 5). The one wire-layer rejection is a fire-mode
/// index the weapon does not offer ([`BadFireMode`](gdtf_qa_protocol::envelope::RejectReason::BadFireMode)).
fn push_actor(
    intent: ActorIntent,
    queues: &mut InjectQueues,
    actors: &InjectActors,
) -> InjectReceipt {
    let Some(actor) = **queues.selected else {
        return InjectReceipt::Queued;
    };
    match intent {
        ActorIntent::Move(dest) => {
            queues
                .pending
                .push(ActIntent::Move(MoveRequested::new(actor, cell_level(dest))));
            InjectReceipt::Queued
        }
        ActorIntent::Turn(facing) => {
            queues.pending.push(ActIntent::Turn(SetFacingRequested::new(
                actor,
                direction(facing),
            )));
            InjectReceipt::Queued
        }
        ActorIntent::Aim(aim) => {
            // The wire carries an EXPLICIT aim flag; the local act is a TOGGLE. Push the
            // toggle only when the actor's current aim differs from the requested one
            // (else it already holds the requested state — a no-op).
            if let Ok(aiming) = actors.aiming.get(actor)
                && **aiming != *aim
            {
                queues.pending.push(ActIntent::AimToggle);
            }
            InjectReceipt::Queued
        }
        ActorIntent::Fire { target, mode } => match resolve_fire_mode(actor, mode, actors) {
            Ok(spec) => {
                let (cell, level) = cell_level(target).split();
                queues.pending.push(ActIntent::Fire(FireRequested::new(
                    actor, spec, cell, level,
                )));
                InjectReceipt::Queued
            }
            Err(reason) => InjectReceipt::Rejected(reason),
        },
    }
}

/// Resolve a contextual act's wire target FAIL-CLOSED, offer-gate it, and push it onto the
/// act's per-act queue (clauses 2/3/4).
///
/// Each arm is one entry in the [`ContextualIntent`] registry: adding a contextual act
/// forces an arm here (no catch-all). A resolve miss is `Rejected(UnknownEntity)`, an
/// unoffered target `Rejected(NotOffered)`, else `Queued`.
fn push_contextual(
    intent: ContextualIntent,
    queues: &mut InjectQueues,
    actors: &InjectActors,
) -> InjectReceipt {
    let outcome = match intent {
        ContextualIntent::Melee(target) => resolve_melee(target, actors).and_then(|resolved| {
            gate_and_push::<MeleeAct>(
                resolved,
                queues.melee_offer.as_deref(),
                &mut queues.melee_queue,
            )
        }),
        ContextualIntent::Shove(token) => resolve_ganger(token, actors).and_then(|resolved| {
            gate_and_push::<ShoveAct>(
                resolved,
                queues.shove_offer.as_deref(),
                &mut queues.shove_queue,
            )
        }),
        ContextualIntent::Stabilize(token) => resolve_ganger(token, actors).and_then(|resolved| {
            gate_and_push::<StabilizeAct>(
                resolved,
                queues.stabilize_offer.as_deref(),
                &mut queues.stabilize_queue,
            )
        }),
        ContextualIntent::Execute(token) => resolve_ganger(token, actors).and_then(|resolved| {
            gate_and_push::<ExecuteAct>(
                resolved,
                queues.execute_offer.as_deref(),
                &mut queues.execute_queue,
            )
        }),
        ContextualIntent::ThrowGrenade(cell) => gate_and_push::<ThrowGrenadeAct>(
            cell_level(cell),
            queues.throw_offer.as_deref(),
            &mut queues.throw_queue,
        ),
        ContextualIntent::OpenDoor(token) => resolve_door(token, actors).and_then(|resolved| {
            gate_and_push::<OpenDoorAct>(
                resolved,
                queues.door_offer.as_deref(),
                &mut queues.door_queue,
            )
        }),
        ContextualIntent::EnterEmplacement(token) => {
            resolve_emplacement(token, actors).and_then(|resolved| {
                gate_and_push::<EnterEmplacementAct>(
                    resolved,
                    queues.enter_offer.as_deref(),
                    &mut queues.enter_queue,
                )
            })
        }
        ContextualIntent::ExitEmplacement(token) => {
            resolve_emplacement(token, actors).and_then(|resolved| {
                gate_and_push::<ExitEmplacementAct>(
                    resolved,
                    queues.exit_offer.as_deref(),
                    &mut queues.exit_queue,
                )
            })
        }
    };
    match outcome {
        Ok(()) => InjectReceipt::Queued,
        Err(reason) => InjectReceipt::Rejected(reason),
    }
}
