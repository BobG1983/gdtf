use bevy::prelude::App;
use gdtf_battle_input::contextual::ThrowGrenadeAct;
use gdtf_qa_command::command::QaCommand;
use gdtf_qa_protocol::command::{CommandAvailability, CommandName, CommandSummary, CommandTiming};

use super::support::{
    ContextualCommand, ContextualReply, ContextualTicket, OfferName, register_contextual,
};
use crate::dev::net_qa::{
    commands::{
        act::support::NoArgs,
        read::{availability::running_and_caught, battle_offers::cell},
    },
    facts::GameFacts,
};

pub(crate) struct ActThrowGrenade;

impl QaCommand for ActThrowGrenade {
    type Args = NoArgs;
    type Facts = GameFacts;
    type Parked = ContextualTicket;
    type Reply = ContextualReply;

    const NAME: CommandName = CommandName::from_static("act.throw_grenade");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Throw at the cell the contextual panel is offering, taking the same path the Throw \
         button does. It takes no target: the panel offers the button while the shooter wields \
         an arcing ranged weapon, and aims at the last cell the cursor hovered. `input.hover` \
         puts the cursor on a pixel and the pick turns that pixel into a cell, but the panel \
         scans for offers before that pick runs, so the cell it offers is the one the frame \
         before resolved. Read `battle.offers` to see which cell came out. A host with no \
         primary window never resolves one at all unless a fixture holds a hover. Refused \
         `NoOffer` when no cell is on offer.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &GameFacts) -> CommandAvailability {
        running_and_caught(*facts)
    }

    fn register_handler(app: &mut App) {
        register_contextual::<Self>(app);
    }
}

impl ContextualCommand for ActThrowGrenade {
    type Act = ThrowGrenadeAct;

    const TARGET: OfferName<ThrowGrenadeAct> = cell;
}
