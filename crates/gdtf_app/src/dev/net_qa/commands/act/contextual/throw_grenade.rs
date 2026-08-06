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
         button does. It takes no target: the panel offers the cell the cursor hovers, and only \
         when the shooter wields an arcing ranged weapon. No QA command holds the cursor on a \
         cell yet — `input.click_cell` writes the hover and the frame's own cursor pick \
         overwrites it — so read `battle.offers` to see whether a cell is on offer. Refused \
         `NoOffer` when none is.",
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
