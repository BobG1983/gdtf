use bevy::prelude::App;
use cobalt_mcp_command::command::QaCommand;
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming,
};
use gdtf_battle_input::contextual::ShoveAct;

use super::support::{
    ContextualCommand, ContextualReply, ContextualTicket, OfferName, register_contextual,
};
use crate::dev::mcp::{
    commands::{
        act::support::NoArgs,
        read::{availability::running_and_caught, battle_offers::ganger},
    },
    facts::GameFacts,
};

pub(crate) struct ActShove;

impl QaCommand for ActShove {
    type Args = NoArgs;
    type Facts = GameFacts;
    type Parked = ContextualTicket;
    type Reply = ContextualReply;

    const NAME: CommandName = CommandName::from_static("act.shove");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Shove the ganger the contextual panel is offering, taking the same path the Shove \
         button does. It takes no target: the panel's scan picks the neighbour, so read \
         `battle.offers` first and move if it picked the wrong one. Refused `NoOffer` when \
         nothing is in reach. The reply brackets the act log with the head before and after \
         the frame.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &GameFacts) -> CommandAvailability {
        running_and_caught(*facts)
    }

    fn register_handler(app: &mut App) {
        register_contextual::<Self>(app);
    }
}

impl ContextualCommand for ActShove {
    type Act = ShoveAct;

    const TARGET: OfferName<ShoveAct> = ganger;
}
