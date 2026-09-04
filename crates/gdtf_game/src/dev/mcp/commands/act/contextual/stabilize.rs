use bevy::prelude::App;
use cobalt_mcp_host::command::McpCommand;
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming,
};
use gdtf_battle_input::contextual::StabilizeAct;

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

pub(crate) struct ActStabilize;

impl McpCommand for ActStabilize {
    type Args = NoArgs;
    type Facts = GameFacts;
    type Parked = ContextualTicket;
    type Reply = ContextualReply;

    const NAME: CommandName = CommandName::from_static("act.stabilize");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Stop the bleeding squadmate the contextual panel is offering, taking the same path the \
         Stabilize button does. It takes no target: the panel's scan picks the neighbour, so \
         read `battle.offers` first and move if it picked the wrong one. Refused `NoOffer` when \
         nothing is in reach. Read the target's bleed state back to see what the sim did.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &GameFacts) -> CommandAvailability {
        running_and_caught(*facts)
    }

    fn register_handler(app: &mut App) {
        register_contextual::<Self>(app);
    }
}

impl ContextualCommand for ActStabilize {
    type Act = StabilizeAct;

    const TARGET: OfferName<StabilizeAct> = ganger;
}
