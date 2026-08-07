use core::time::Duration;

use bevy::prelude::*;
use gdtf_battle_input::ActIntent;
use gdtf_battle_sim::{battle::PlayerFaction, turn::ActiveFaction};
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_command::{
    command::QaCommand,
    dispatch::{CommandCall, DeferredBudget, DeferredReplies, take_calls},
};
use gdtf_qa_protocol::command::{CommandAvailability, CommandName, CommandSummary, CommandTiming};

use super::{
    sets::ActCommandSystems,
    support::{ActClaim, ActSettle, NoArgs},
};
use crate::dev::net_qa::{
    commands::{
        read::availability::{running_and_caught, turn_is_the_players},
        wait::probe::TurnChangeCount,
    },
    facts::GameFacts,
    wire::act::{ActReply, ActSeqNet},
};

/// What one parked `act.end_turn` is holding out for.
pub(crate) struct EndTurnTicket {
    from:        ActSeqNet,
    parked_with: TurnChangeCount,
}

pub(crate) struct ActEndTurn;

impl QaCommand for ActEndTurn {
    type Args = NoArgs;
    type Facts = GameFacts;
    type Parked = EndTurnTicket;
    type Reply = ActReply;

    const DEFERRED_BUDGET: DeferredBudget = DeferredBudget::new(Duration::from_secs(30));
    const NAME: CommandName = CommandName::from_static("act.end_turn");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "End the acting gang's turn, taking the same path the end-turn button does. It refuses \
         `WrongState` while another faction is acting, because no player path ends someone \
         else's turn — wait on `TurnChanged` to get past an enemy turn. The reply is held back \
         until the turn comes round to the player again, so it lands after the enemy has \
         finished acting, and brackets the act log across everything that happened in between. \
         `complete` is always true here: the command has no single actor to still be walking an \
         act out, and the reply only lands once the turn is back with the player.",
    );
    const TIMING: CommandTiming = CommandTiming::Deferred;

    fn availability(facts: &GameFacts) -> CommandAvailability {
        match running_and_caught(*facts) {
            CommandAvailability::Available => turn_is_the_players(*facts),
            refused @ CommandAvailability::Unavailable { .. } => refused,
        }
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            (
                claim_act_end_turn.in_set(ActCommandSystems::Claim),
                settle_act_end_turn.in_set(ActCommandSystems::Settle),
            ),
        );
    }
}

fn claim_act_end_turn(
    mut queue: ResMut<PendingQueue<CommandCall<ActEndTurn>>>,
    mut deferred: ResMut<DeferredReplies<ActEndTurn>>,
    mut claim: ActClaim,
    turns: Res<TurnChangeCount>,
) {
    if queue.is_empty() {
        return;
    }
    let from = claim.head();
    let parked_with = *turns;
    for (_args, responder) in take_calls::<ActEndTurn>(&mut queue) {
        claim.push(ActIntent::EndTurn);
        deferred.park(responder, EndTurnTicket { from, parked_with });
    }
}

fn settle_act_end_turn(
    settle: ActSettle,
    turns: Res<TurnChangeCount>,
    active: Option<Res<ActiveFaction>>,
    player: Option<Res<PlayerFaction>>,
    mut deferred: ResMut<DeferredReplies<ActEndTurn>>,
) {
    if deferred.is_empty() {
        return;
    }
    let acting = active.map(|active| **active);
    let commanded = player.map(|player| **player);
    let players_turn = acting.is_some() && acting == commanded;
    let delivered = deferred.answer_resolved(|ticket| {
        // No single actor to watch, so the window always reports itself complete.
        (players_turn && **turns > *ticket.parked_with).then(|| settle.window(ticket.from, None))
    });
    if *delivered > 0 {
        debug!(
            delivered = *delivered,
            "net_qa: act.end_turn settled once the turn came back to the player"
        );
    }
}
