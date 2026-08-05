use core::time::Duration;

use bevy::prelude::*;
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_command::{
    command::QaCommand,
    dispatch::{CommandCall, DeferredBudget, DeferredReplies, QaCommandSystems, take_calls},
};
use gdtf_qa_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use serde::{Deserialize, Serialize};

use crate::{
    dev::net_qa::{
        facts::{BattleActivity, GameFacts, GameFactsParam},
        wire::AppPhaseNet,
    },
    states::running::game::battlescape::battle_running::insert_battle_running_complete,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BattleFleeArgs {}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct BattleFleeReply {
    phase: AppPhaseNet,
}

pub(crate) struct BattleFlee;

impl QaCommand for BattleFlee {
    type Args = BattleFleeArgs;
    type Facts = GameFacts;
    type Parked = ();
    type Reply = BattleFleeReply;

    const DEFERRED_BUDGET: DeferredBudget = DeferredBudget::new(Duration::from_secs(30));
    const NAME: CommandName = CommandName::from_static("battle.flee");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Walk away from the running battle, taking the same path the Flee button does. Needs a \
         battle in its running phase; the reply arrives once the battle has left it and carries \
         the phase the app landed in.",
    );
    const TIMING: CommandTiming = CommandTiming::Deferred;

    fn availability(facts: &GameFacts) -> CommandAvailability {
        match facts.battle_activity() {
            BattleActivity::Running => CommandAvailability::Available,
            BattleActivity::NotRunning => CommandAvailability::Unavailable {
                code: UnavailableCode::WrongState,
                note: RefusalNote::from_static(
                    "battle.flee leaves a running battle, and no battle is running",
                ),
            },
        }
    }

    fn register_handler(app: &mut App) {
        app.add_systems(Update, handle_battle_flee.after(QaCommandSystems::Claim));
    }
}

fn handle_battle_flee(
    facts: GameFactsParam,
    mut queue: ResMut<PendingQueue<CommandCall<BattleFlee>>>,
    mut deferred: ResMut<DeferredReplies<BattleFlee>>,
    mut commands: Commands,
) {
    if !queue.is_empty() {
        for (_args, responder) in take_calls::<BattleFlee>(&mut queue) {
            insert_battle_running_complete(&mut commands);
            deferred.park(responder, ());
        }
    }
    if deferred.is_empty() {
        return;
    }
    let sampled = facts.sample();
    if sampled.battle_activity() == BattleActivity::Running {
        return;
    }
    let delivered = deferred.answer_all(&BattleFleeReply {
        phase: sampled.phase(),
    });
    debug!(
        delivered = *delivered,
        "net_qa: battle.flee settled once the battle left its running phase"
    );
}
