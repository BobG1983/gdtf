use bevy::prelude::App;
use gdtf_battle_input::contextual::MeleeAct;
use gdtf_qa_command::command::QaCommand;
use gdtf_qa_protocol::command::{CommandAvailability, CommandName, CommandSummary, CommandTiming};

use super::support::{
    ContextualCommand, ContextualReply, ContextualTicket, OfferName, register_contextual,
};
use crate::dev::net_qa::{
    commands::{
        act::support::NoArgs,
        read::{availability::running_and_caught, battle_offers::melee},
    },
    facts::GameFacts,
};

pub(crate) struct ActMelee;

impl QaCommand for ActMelee {
    type Args = NoArgs;
    type Facts = GameFacts;
    type Parked = ContextualTicket;
    type Reply = ContextualReply;

    const NAME: CommandName = CommandName::from_static("act.melee");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Swing at whatever the contextual panel is offering, taking the same path the Melee \
         button does. It takes no target: the panel's scan picks the neighbour or the structure \
         cell, so read `battle.offers` first and move if it picked the wrong one. Refused \
         `NoOffer` when nothing is in reach. The reply brackets the act log with the head \
         before and after the frame.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &GameFacts) -> CommandAvailability {
        running_and_caught(*facts)
    }

    fn register_handler(app: &mut App) {
        register_contextual::<Self>(app);
    }
}

impl ContextualCommand for ActMelee {
    type Act = MeleeAct;

    const TARGET: OfferName<MeleeAct> = melee;
}
