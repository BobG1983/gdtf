use bevy::prelude::*;
use cobalt_mcp_command::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming,
};
use cobalt_mcp_transport::PendingQueue;
use gdtf_battle_sim::{battle::PlayerFaction, turn::ActiveFaction};
use serde::{Deserialize, Serialize};

use super::availability::on_the_battle_screen;
use crate::dev::mcp::{facts::GameFacts, wire::roster::FactionNet};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BattleTurnArgs {}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct BattleTurnReply {
    active: Option<FactionNet>,
    player: Option<FactionNet>,
}

pub(crate) struct BattleTurn;

impl QaCommand for BattleTurn {
    type Args = BattleTurnArgs;
    type Facts = GameFacts;
    type Parked = ();
    type Reply = BattleTurnReply;

    const NAME: CommandName = CommandName::from_static("battle.turn");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Read which gang is acting, and which gang the player commands. Both are absent until \
         the battle has generated its factions, so early in the battlescape this answers with \
         nothing rather than refusing.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &GameFacts) -> CommandAvailability {
        on_the_battle_screen(*facts)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(Update, handle_battle_turn.after(QaCommandSystems::Claim));
    }
}

fn handle_battle_turn(
    active: Option<Res<ActiveFaction>>,
    player: Option<Res<PlayerFaction>>,
    mut queue: ResMut<PendingQueue<CommandCall<BattleTurn>>>,
) {
    if queue.is_empty() {
        return;
    }
    let reply = BattleTurnReply {
        active: active.map(|faction| FactionNet::from_sim(**faction)),
        player: player.map(|faction| FactionNet::from_sim(**faction)),
    };
    for (_args, responder) in take_calls::<BattleTurn>(&mut queue) {
        responder.answer(&reply);
    }
}
