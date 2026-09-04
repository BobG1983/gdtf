use bevy::prelude::App;
use cobalt_mcp_command::command::McpCommand;
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming,
};
use gdtf_battle_input::contextual::OpenDoorAct;

use super::support::{
    ContextualCommand, ContextualReply, ContextualTicket, OfferName, register_contextual,
};
use crate::dev::mcp::{
    commands::{
        act::support::NoArgs,
        read::{availability::running_and_caught, battle_offers::door},
    },
    facts::GameFacts,
};

pub(crate) struct ActOpenDoor;

impl McpCommand for ActOpenDoor {
    type Args = NoArgs;
    type Facts = GameFacts;
    type Parked = ContextualTicket;
    type Reply = ContextualReply;

    const NAME: CommandName = CommandName::from_static("act.open_door");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Open the door the contextual panel is offering, taking the same path the Open Door \
         button does. It takes no target: the panel's scan picks the door, so read \
         `battle.offers` first and move if it picked the wrong one. Refused `NoOffer` when no \
         door is on offer. The act log has no deed for a door, so an empty window is correct \
         here — read the door's state instead.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &GameFacts) -> CommandAvailability {
        running_and_caught(*facts)
    }

    fn register_handler(app: &mut App) {
        register_contextual::<Self>(app);
    }
}

impl ContextualCommand for ActOpenDoor {
    type Act = OpenDoorAct;

    const TARGET: OfferName<OpenDoorAct> = door;
}
