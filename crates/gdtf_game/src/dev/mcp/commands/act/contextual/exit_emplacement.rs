use bevy::prelude::App;
use cobalt_mcp_host::command::McpCommand;
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming,
};
use gdtf_battle_input::contextual::ExitEmplacementAct;

use super::support::{
    ContextualCommand, ContextualReply, ContextualTicket, OfferName, register_contextual,
};
use crate::dev::mcp::{
    commands::{
        act::support::NoArgs,
        read::{availability::running_and_caught, battle_offers::emplacement},
    },
    facts::GameFacts,
};

pub(crate) struct ActExitEmplacement;

impl McpCommand for ActExitEmplacement {
    type Args = NoArgs;
    type Facts = GameFacts;
    type Parked = ContextualTicket;
    type Reply = ContextualReply;

    const NAME: CommandName = CommandName::from_static("act.exit_emplacement");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Dismount the emplacement the contextual panel is offering, taking the same path the \
         Exit button does. It takes no target: the panel offers the emplacement the selected \
         shooter is riding. Refused `NoOffer` when the shooter is riding none. The act log has \
         no deed for an emplacement, so an empty window is correct here — read the \
         emplacement's occupancy instead.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &GameFacts) -> CommandAvailability {
        running_and_caught(*facts)
    }

    fn register_handler(app: &mut App) {
        register_contextual::<Self>(app);
    }
}

impl ContextualCommand for ActExitEmplacement {
    type Act = ExitEmplacementAct;

    const TARGET: OfferName<ExitEmplacementAct> = emplacement;
}
