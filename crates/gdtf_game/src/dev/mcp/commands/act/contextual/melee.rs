use bevy::prelude::App;
use cobalt_mcp_command::command::QaCommand;
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming,
};
use gdtf_battle_input::contextual::MeleeAct;
use serde::Deserialize;

use super::support::{
    ContextualCommand, ContextualReply, ContextualTicket, OfferName, register_contextual,
};
use crate::dev::mcp::{
    commands::read::{availability::running_and_caught, battle_offers::melee},
    facts::GameFacts,
    wire::{act::ActRefusalNet, act_payload::MeleeTargetNet, offer::OfferTargetNet},
};

/// What `act.melee` takes: the target the strike is meant for.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActMeleeArgs {
    /// The target to swing at, as `battle.cost` prices it and `battle.offers` reports it.
    target: MeleeTargetNet,
}

/// The melee target as the wire names an offer, so the two compare as one value.
const fn as_offered(target: MeleeTargetNet) -> OfferTargetNet {
    match target {
        MeleeTargetNet::Ganger(token) => OfferTargetNet::Ganger(token),
        MeleeTargetNet::Structure(at) => OfferTargetNet::Cell(at),
    }
}

pub(crate) struct ActMelee;

impl QaCommand for ActMelee {
    type Args = ActMeleeArgs;
    type Facts = GameFacts;
    type Parked = ContextualTicket;
    type Reply = ContextualReply;

    const NAME: CommandName = CommandName::from_static("act.melee");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Swing at the target you name, taking the same path the Melee button does. Takes \
         `(target: Ganger(...))` or `(target: Structure(...))` — the same value `battle.cost \
         {Melee}` prices and `battle.offers` reports, so read one of those first. Refused \
         `TargetMismatch` when the panel is offering a different target, which is what stops a \
         strike the quote refused from landing on a wall instead, and `NoOffer` when nothing is \
         in reach. The reply brackets the act log with the head before and after the frame.",
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

    fn target_refusal(args: &ActMeleeArgs, offered: OfferTargetNet) -> Option<ActRefusalNet> {
        (as_offered(args.target) != offered).then_some(ActRefusalNet::TargetMismatch)
    }
}
