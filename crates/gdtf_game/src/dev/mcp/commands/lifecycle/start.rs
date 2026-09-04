use core::time::Duration;

use bevy::prelude::*;
use cobalt_mcp_command::{
    command::McpCommand,
    dispatch::{CommandCall, DeferredBudget, DeferredReplies, McpCommandSystems, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use cobalt_mcp_transport::PendingQueue;
use gdtf_battle_sim::rng::BattleSeed;
use serde::{Deserialize, Serialize};

use crate::{
    dev::mcp::{
        facts::GameFacts,
        wire::{RunningPhaseNet, misc::SeedNet},
    },
    states::running::{
        game::battlescape::generation::{GenerationComplete, battle_sim::ResolvedBattleSeed},
        menu::StartBattleRequested,
    },
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BattleStartArgs {
    #[serde(default)]
    seed: Option<SeedNet>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct BattleStartReply {
    seed: SeedNet,
}

pub(crate) struct BattleStart;

impl McpCommand for BattleStart {
    type Args = BattleStartArgs;
    type Facts = GameFacts;
    type Parked = ();
    type Reply = BattleStartReply;

    const DEFERRED_BUDGET: DeferredBudget = DeferredBudget::new(Duration::from_secs(30));
    const NAME: CommandName = CommandName::from_static("battle.start");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Leave the menu and start a battle, taking the same path the Battlescape button does. \
         The seed is the only input and is optional; omit it and the game resolves its own. The \
         reply arrives once generation has finished and carries the seed the battle actually \
         used.",
    );
    const TIMING: CommandTiming = CommandTiming::Deferred;

    fn availability(facts: &GameFacts) -> CommandAvailability {
        match facts.phase().running() {
            Some(RunningPhaseNet::Menu) => CommandAvailability::Available,
            Some(_) | None => CommandAvailability::Unavailable {
                code: UnavailableCode::WrongState,
                note: RefusalNote::from_static(
                    "battle.start starts a battle from the main menu, and the app is not there",
                ),
            },
        }
    }

    fn register_handler(app: &mut App) {
        app.add_systems(Update, handle_battle_start.after(McpCommandSystems::Claim));
    }
}

fn handle_battle_start(
    mut queue: ResMut<PendingQueue<CommandCall<BattleStart>>>,
    mut deferred: ResMut<DeferredReplies<BattleStart>>,
    mut requests: MessageWriter<StartBattleRequested>,
    resolved: Option<Res<ResolvedBattleSeed>>,
    generated: Option<Res<GenerationComplete>>,
) {
    if !queue.is_empty() {
        for (args, responder) in take_calls::<BattleStart>(&mut queue) {
            requests.write(StartBattleRequested::new(
                args.seed.map(|seed| BattleSeed::new(*seed)),
            ));
            deferred.park(responder, ());
        }
    }
    if deferred.is_empty() || generated.is_none() {
        return;
    }
    let Some(resolved) = resolved else {
        return;
    };
    let delivered = deferred.answer_all(&BattleStartReply {
        seed: SeedNet::new(resolved.get()),
    });
    debug!(
        delivered = *delivered,
        seed = resolved.get(),
        "mcp: battle.start settled once generation finished"
    );
}
