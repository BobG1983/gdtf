//! `battle.cost` — the TU cost and legality of one act, before it is taken.

use bevy::prelude::*;
use cobalt_mcp_command::{
    command::McpCommand,
    dispatch::{CommandCall, McpCommandSystems, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use cobalt_mcp_transport::PendingQueue;
use serde::{Deserialize, Serialize};

use super::{
    price::quote,
    reads::{CostRows, CostWorld},
};
use crate::dev::mcp::{
    commands::read::availability::battle_is_running,
    facts::GameFacts,
    wire::{
        cost::{CostActNet, CostLegalNet, CostRefusalNet},
        token::GangerToken,
        vitals::TuNet,
    },
};

/// The refusal a call gets while the battle's sim resources are not up.
const NO_MODEL: RefusalNote = RefusalNote::from_static(
    "the battle's tuning, grids, fog and cover are not loaded, so the sim has no cost to quote",
);

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BattleCostArgs {
    /// Token of the ganger that would act, as `battle.roster` reports it.
    actor: GangerToken,
    /// The act to price, carrying whatever it would act on.
    act:   CostActNet,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct BattleCostReply {
    cost:    Option<TuNet>,
    legal:   CostLegalNet,
    refusal: Option<CostRefusalNet>,
}

impl BattleCostReply {
    /// Build from what the sim quoted. `legal` follows from the refusal and is never passed in.
    const fn new(cost: Option<TuNet>, refusal: Option<CostRefusalNet>) -> Self {
        Self {
            cost,
            legal: CostLegalNet::from_refusal(refusal),
            refusal,
        }
    }
}

pub(crate) struct BattleCost;

impl McpCommand for BattleCost {
    type Args = BattleCostArgs;
    type Facts = GameFacts;
    type Parked = ();
    type Reply = BattleCostReply;

    const NAME: CommandName = CommandName::from_static("battle.cost");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Ask what one act would cost a ganger in TU and whether the sim would allow it, without \
         taking it. Every number is the sim's own cost helper and every verdict its own legality \
         check. What this quotes is the price of the whole act; a move stopped part-way charges \
         only the steps taken. It reads only — nothing in the battle moves. A refusal names why: \
         an unknown token, a ganger the player does not command, a cell with no route, \
         suppression the walk does not break away from, cover between a striker and its target, \
         a pool that cannot pay, or an act the sim rejects.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &GameFacts) -> CommandAvailability {
        battle_is_running(*facts)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(Update, handle_battle_cost.after(McpCommandSystems::Claim));
    }
}

fn handle_battle_cost(
    world: CostWorld,
    rows: CostRows,
    mut queue: ResMut<PendingQueue<CommandCall<BattleCost>>>,
) {
    if queue.is_empty() {
        return;
    }
    let Some(loaded) = world.loaded() else {
        for (_args, responder) in take_calls::<BattleCost>(&mut queue) {
            responder.unavailable(UnavailableCode::MissingModel, NO_MODEL);
        }
        return;
    };
    for (args, responder) in take_calls::<BattleCost>(&mut queue) {
        let quoted = quote(&loaded, &rows, args.actor, args.act);
        responder.answer(&BattleCostReply::new(
            quoted.cost().map(|tu| TuNet::new(*tu)),
            quoted.refusal(),
        ));
    }
}

#[cfg(test)]
mod test {
    use super::{BattleCostReply, CostRefusalNet, TuNet};

    const REFUSALS: [CostRefusalNet; 6] = [
        CostRefusalNet::NoSuchGanger,
        CostRefusalNet::NotYourGanger,
        CostRefusalNet::NoPathToCell,
        CostRefusalNet::Suppressed,
        CostRefusalNet::CannotAfford,
        CostRefusalNet::ActNotAllowed,
    ];

    #[test]
    fn the_reply_is_legal_exactly_when_no_refusal_was_given() {
        let allowed = BattleCostReply::new(Some(TuNet::new(9)), None);
        assert!(
            *allowed.legal,
            "an act nothing refused must answer legal true",
        );
        for refusal in REFUSALS {
            let reply = BattleCostReply::new(None, Some(refusal));
            assert!(
                !*reply.legal,
                "{refusal:?} must answer legal false — legal is derived, never passed in",
            );
        }
    }
}
