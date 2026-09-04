//! Pieces every contextual act command is built from.

use bevy::{ecs::system::SystemParam, prelude::*};
use cobalt_mcp_command::{
    command::McpCommand,
    dispatch::{CommandCall, DeferredReplies, take_calls},
};
use cobalt_mcp_transport::PendingQueue;
use gdtf_battle_input::{
    SelectedShooter,
    contextual::{ContextualAct, PendingContextualIntents},
};
use gdtf_battle_sim::act_log::ActLog;
use serde::{Deserialize, Serialize};

use crate::{
    dev::mcp::{
        commands::act::{
            sets::ActCommandSystems,
            support::{ActSettle, ActTicket, head_of},
        },
        wire::{
            act::{ActCompleteNet, ActRefusalNet, ActSeqNet},
            offer::OfferTargetNet,
        },
    },
    states::running::game::battlescape::contextual_panel::ContextualOffer,
};

crate::support_item! {
    /// What every contextual act answers: the act-log window it opened and what it fired at.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    enum ContextualReply {
        /// The panel's target reached the sim; read `from_seq..to_seq` from the act log.
        Accepted {
            /// Act-log head when the call was claimed.
            from_seq: ActSeqNet,
            /// Act-log head once the sim had run that frame.
            to_seq:   ActSeqNet,
            /// False while the actor is still walking the act out.
            complete: ActCompleteNet,
            /// The target the panel was offering, which is the one the call fired at.
            target:   OfferTargetNet,
        },
        /// The call never reached the sim.
        Refused {
            /// Why it was turned away.
            reason: ActRefusalNet,
        },
    }
}

/// The refusal a contextual act answers when its family is offering nothing this frame.
pub(crate) const NO_OFFER: ContextualReply = ContextualReply::Refused {
    reason: ActRefusalNet::NoOffer,
};

/// How one act family's offer is named on the wire — the same mapper `battle.offers` uses.
pub(crate) type OfferName<A> = fn(&ContextualOffer<A>) -> Option<OfferTargetNet>;

/// A command that fires one act family at whatever the contextual panel is offering it.
pub(crate) trait ContextualCommand:
    McpCommand<Parked = ContextualTicket, Reply = ContextualReply>
{
    /// The act family this command fires, which is the only one it reads or pushes onto.
    type Act: ContextualAct;

    /// How that family's target is named on the wire.
    const TARGET: OfferName<Self::Act>;

    /// Why the call's arguments do not name the offered target, or nothing when they do.
    /// The default takes whatever is offered, which is what a command with no target wants.
    fn target_refusal(_args: &Self::Args, _offered: OfferTargetNet) -> Option<ActRefusalNet> {
        None
    }
}

/// What one parked contextual call needs to report its window and what it fired at.
pub(crate) struct ContextualTicket {
    act:    ActTicket,
    target: OfferTargetNet,
}

impl ContextualTicket {
    /// Park with the classic act ticket and the target the panel offered.
    const fn new(act: ActTicket, target: OfferTargetNet) -> Self {
        Self { act, target }
    }

    fn window(&self, settle: &ActSettle) -> ContextualReply {
        ContextualReply::Accepted {
            from_seq: self.act.claimed_at(),
            to_seq:   settle.head(),
            complete: settle.completed(self.act.actor()),
            target:   self.target,
        }
    }
}

/// What a contextual act command reads and writes as it claims its calls.
#[derive(SystemParam)]
pub(crate) struct ContextualClaim<'w, A: ContextualAct> {
    log:      Option<Res<'w, ActLog>>,
    selected: Option<Res<'w, SelectedShooter>>,
    offer:    Option<Res<'w, ContextualOffer<A>>>,
    pending:  Option<ResMut<'w, PendingContextualIntents<A>>>,
}

impl<A: ContextualAct> ContextualClaim<'_, A> {
    /// The act-log head as it stands before the sim runs this frame.
    fn head(&self) -> ActSeqNet {
        head_of(self.log.as_deref())
    }

    /// The selected shooter, when the game has one.
    fn shooter(&self) -> Option<Entity> {
        self.selected.as_deref().and_then(|selected| **selected)
    }

    /// What the panel is offering this frame, as the sim holds it and as the wire names it.
    fn offered(&self, name: OfferName<A>) -> Option<(A::Target, OfferTargetNet)> {
        let offer = self.offer.as_deref()?;
        let target = offer.target()?;
        let named = name(offer)?;
        Some((target, named))
    }

    /// Queue the offered target onto the queue a button press pushes onto.
    fn fire(&mut self, target: A::Target) -> Option<()> {
        self.pending.as_mut()?.push(target);
        Some(())
    }
}

/// Register one contextual act command's two systems in the bands the act commands share.
pub(crate) fn register_contextual<C: ContextualCommand>(app: &mut App) {
    app.add_systems(
        Update,
        (
            claim_contextual::<C>.in_set(ActCommandSystems::ContextualClaim),
            settle_contextual::<C>.in_set(ActCommandSystems::Settle),
        ),
    );
}

/// Push this family's offered target once per claimed call, and park each caller's reply.
fn claim_contextual<C: ContextualCommand>(
    mut queue: ResMut<PendingQueue<CommandCall<C>>>,
    mut deferred: ResMut<DeferredReplies<C>>,
    mut claim: ContextualClaim<C::Act>,
) {
    if queue.is_empty() {
        return;
    }
    let from = claim.head();
    let actor = claim.shooter();
    for (args, responder) in take_calls::<C>(&mut queue) {
        let Some((target, named)) = claim.offered(C::TARGET) else {
            responder.answer(&NO_OFFER);
            continue;
        };
        if let Some(reason) = C::target_refusal(&args, named) {
            responder.answer(&ContextualReply::Refused { reason });
            continue;
        }
        if claim.fire(target).is_none() {
            responder.answer(&NO_OFFER);
            continue;
        }
        deferred.park(
            responder,
            ContextualTicket::new(ActTicket::new(from, actor), named),
        );
    }
}

/// Answer every contextual call parked this frame with the window the sim just closed.
fn settle_contextual<C: ContextualCommand>(
    settle: ActSettle,
    mut deferred: ResMut<DeferredReplies<C>>,
) {
    if deferred.is_empty() {
        return;
    }
    let delivered = deferred.answer_resolved(|ticket| Some(ticket.window(&settle)));
    debug!(
        command = C::NAME.as_str(),
        delivered = *delivered,
        "mcp: a contextual act reported the log window it opened"
    );
}
