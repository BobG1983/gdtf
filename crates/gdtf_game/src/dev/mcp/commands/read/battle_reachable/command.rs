//! `battle.reachable` lists every cell one ganger can walk to, and what each one charges.

use bevy::prelude::*;
use cobalt_mcp_host::{
    PendingQueue,
    command::McpCommand,
    dispatch::{CommandCall, McpCommandSystems, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, UnavailableCode,
};
use gdtf_battle_presenter::{PresenterSystems, promote_shown_fog};
use serde::{Deserialize, Serialize};

use super::reads::{ReachableRows, ReachableScene, ReachableTerrain};
use crate::{
    dev::mcp::{
        commands::{
            act::support::a_ganger,
            read::{availability::presenter_is_ready, shown::ShownBattleReads},
        },
        facts::GameFacts,
        wire::{
            cell::CellLevelNet, cost::CostRefusalNet, reachable::ReachableCellNet,
            token::GangerToken, vitals::TuNet,
        },
    },
    states::running::game::battlescape::inspect_panel::shadow::promote_shown_occupancy,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BattleReachableArgs {
    /// Token of the ganger whose reachable cells are asked for, as `battle.roster` reports it.
    actor: GangerToken,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct BattleReachableReply {
    cells:   Option<Vec<ReachableCellNet>>,
    refusal: Option<CostRefusalNet>,
}

impl BattleReachableReply {
    /// Report the cells the search returned, in the order it returned them.
    const fn reached(cells: Vec<ReachableCellNet>) -> Self {
        Self {
            cells:   Some(cells),
            refusal: None,
        }
    }

    /// Refuse without a cell list, so an empty result never reads as a refusal.
    const fn refused(refusal: CostRefusalNet) -> Self {
        Self {
            cells:   None,
            refusal: Some(refusal),
        }
    }
}

pub(crate) struct BattleReachable;

impl McpCommand for BattleReachable {
    type Args = BattleReachableArgs;
    type Facts = GameFacts;
    type Parked = ();
    type Reply = BattleReachableReply;

    const NAME: CommandName = CommandName::from_static("battle.reachable");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "List every cell the named ganger can walk to inside its TU pool, each with the TU \
         reaching it charges. The search is the sim's own, run over the picture on screen: the \
         cell the sprite is drawn on, the pool the screen is showing, the grid the screen has \
         drawn and the fog the screen is lighting. It reads only. Nothing in the battle moves. \
         A refusal names why: an unknown token, or a ganger the player does not command.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &GameFacts) -> CommandAvailability {
        presenter_is_ready(*facts)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_battle_reachable
                .after(McpCommandSystems::Claim)
                .after(PresenterSystems::Compose)
                .after(promote_shown_occupancy)
                .after(promote_shown_fog),
        );
    }
}

fn handle_battle_reachable(
    reads: ShownBattleReads,
    terrain: ReachableTerrain,
    rows: ReachableRows,
    mut queue: ResMut<PendingQueue<CommandCall<BattleReachable>>>,
) {
    if queue.is_empty() {
        return;
    }
    let scene = match ReachableScene::assemble(reads.shown(), &terrain) {
        Ok(scene) => scene,
        Err(note) => {
            for (_args, responder) in take_calls::<BattleReachable>(&mut queue) {
                responder.unavailable(UnavailableCode::MissingModel, note.clone());
            }
            return;
        }
    };
    for (args, responder) in take_calls::<BattleReachable>(&mut queue) {
        responder.answer(&answer_for(&scene, &rows, args.actor));
    }
}

/// What one call is answered with: the cells the search reached, or the refusal it earned.
fn answer_for(
    scene: &ReachableScene<'_>,
    rows: &ReachableRows<'_, '_>,
    actor: GangerToken,
) -> BattleReachableReply {
    let Some(entity) = a_ganger(&rows.tokens, actor) else {
        return BattleReachableReply::refused(CostRefusalNet::NoSuchGanger);
    };
    let Ok(row) = rows.actors.get(entity) else {
        return BattleReachableReply::refused(CostRefusalNet::NoSuchGanger);
    };
    if *row.faction != scene.player {
        return BattleReachableReply::refused(CostRefusalNet::NotYourGanger);
    }
    BattleReachableReply::reached(
        scene
            .reached(rows, &row)
            .into_iter()
            .map(|(at, cost)| ReachableCellNet::new(CellLevelNet::from_sim(at), TuNet::new(*cost)))
            .collect(),
    )
}
