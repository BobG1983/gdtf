use bevy::prelude::*;
use gdtf_battle_presenter::PresenterSystems;
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_command::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};
use gdtf_qa_protocol::command::{CommandAvailability, CommandName, CommandSummary, CommandTiming};
use serde::{Deserialize, Serialize};

use super::{
    availability::presenter_is_ready,
    shown::{ShownBattleReads, ganger_card},
};
use crate::{
    dev::net_qa::{
        facts::GameFacts,
        wire::{
            cell::CellLevelNet,
            inspect::{CoverBlockNet, InspectShownNet},
        },
    },
    states::running::game::battlescape::{
        inspect_panel::{
            decide::{InspectShown, inspect_shown},
            shadow::{promote_shown_cover, promote_shown_occupancy},
        },
        stat_block::StatBlockData,
    },
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BattleInspectArgs {
    /// The cell to read, as `battle.selection` and `battle.visible` report cells.
    at: CellLevelNet,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct BattleInspectReply {
    at:    CellLevelNet,
    shown: InspectShownNet,
}

pub(crate) struct BattleInspect;

impl QaCommand for BattleInspect {
    type Args = BattleInspectArgs;
    type Facts = GameFacts;
    type Parked = ();
    type Reply = BattleInspectReply;

    const NAME: CommandName = CommandName::from_static("battle.inspect");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Read one cell exactly as the inspect panel would draw it: a squad-visible ganger's \
         card, else the cover block for destructible terrain, else nothing. Takes the cell to \
         read; the panel's own hovered and pinned cells come from battle.selection.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &GameFacts) -> CommandAvailability {
        presenter_is_ready(*facts)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_battle_inspect
                .after(QaCommandSystems::Claim)
                .after(PresenterSystems::Compose)
                .after(promote_shown_occupancy)
                .after(promote_shown_cover),
        );
    }
}

fn handle_battle_inspect(
    reads: ShownBattleReads,
    rows: Query<StatBlockData>,
    mut queue: ResMut<PendingQueue<CommandCall<BattleInspect>>>,
) {
    if queue.is_empty() {
        return;
    }
    let shown = reads.shown();
    for (args, responder) in take_calls::<BattleInspect>(&mut queue) {
        let at = args.at.to_sim();
        let decision = inspect_shown(Some(at), shown, |entity| {
            rows.get(entity).ok().map(|row| *row.faction)
        });
        responder.answer(&BattleInspectReply {
            at:    args.at,
            shown: on_the_wire(decision, args.at, &rows),
        });
    }
}

fn on_the_wire(
    decision: InspectShown,
    at: CellLevelNet,
    rows: &Query<StatBlockData>,
) -> InspectShownNet {
    match decision {
        InspectShown::Ganger(entity) => match rows.get(entity) {
            Ok(row) => InspectShownNet::Ganger(ganger_card(entity, at, &row)),
            Err(_) => InspectShownNet::Nothing,
        },
        InspectShown::Cover(entry) => InspectShownNet::Cover(CoverBlockNet::from_sim(entry)),
        InspectShown::Nothing => InspectShownNet::Nothing,
    }
}
