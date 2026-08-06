use bevy::prelude::App;
use gdtf_battle_input::contextual::EnterEmplacementAct;
use gdtf_qa_command::command::QaCommand;
use gdtf_qa_protocol::command::{CommandAvailability, CommandName, CommandSummary, CommandTiming};

use super::support::{
    ContextualCommand, ContextualReply, ContextualTicket, OfferName, register_contextual,
};
use crate::dev::net_qa::{
    commands::{
        act::support::NoArgs,
        read::{availability::running_and_caught, battle_offers::emplacement},
    },
    facts::GameFacts,
};

pub(crate) struct ActEnterEmplacement;

impl QaCommand for ActEnterEmplacement {
    type Args = NoArgs;
    type Facts = GameFacts;
    type Parked = ContextualTicket;
    type Reply = ContextualReply;

    const NAME: CommandName = CommandName::from_static("act.enter_emplacement");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Mount the emplacement the contextual panel is offering, taking the same path the Enter \
         button does. It takes no target: the panel's scan picks the emplacement, so read \
         `battle.offers` first and move if it picked the wrong one. Refused `NoOffer` when none \
         is in reach. The act log has no deed for an emplacement, so an empty window is correct \
         here — read the emplacement's occupancy instead.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &GameFacts) -> CommandAvailability {
        running_and_caught(*facts)
    }

    fn register_handler(app: &mut App) {
        register_contextual::<Self>(app);
    }
}

impl ContextualCommand for ActEnterEmplacement {
    type Act = EnterEmplacementAct;

    const TARGET: OfferName<EnterEmplacementAct> = emplacement;
}
