use bevy::prelude::*;
use gdtf_battle_presenter::PresenterSystems;
use gdtf_battle_sim::emplacement::Mounted;
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_command::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};
use gdtf_qa_protocol::command::{CommandAvailability, CommandName, CommandSummary, CommandTiming};
use serde::{Deserialize, Serialize};

use super::{
    availability::on_the_battle_screen,
    shown::{DrawnCell, ShownBattleReads, ganger_card},
};
use crate::{
    dev::net_qa::{
        facts::GameFacts,
        wire::{cell::CellLevelNet, roster::GangerCardNet},
    },
    states::running::game::battlescape::{
        inspect_panel::shadow::{promote_shown_cover, promote_shown_occupancy},
        stat_block::StatBlockData,
    },
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BattleRosterArgs {}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct BattleRosterReply {
    gangers: Vec<GangerCardNet>,
}

pub(crate) struct BattleRoster;

impl QaCommand for BattleRoster {
    type Args = BattleRosterArgs;
    type Facts = GameFacts;
    type Parked = ();
    type Reply = BattleRosterReply;

    const NAME: CommandName = CommandName::from_static("battle.roster");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Read every player ganger's card plus the cards of enemies the squad can currently see, \
         each on the cell its sprite is drawn on. Cards carry the values the stat block is \
         drawing, so they lag the sim while an act plays out, exactly as the screen does.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &GameFacts) -> CommandAvailability {
        on_the_battle_screen(*facts)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_battle_roster
                .after(QaCommandSystems::Claim)
                .after(PresenterSystems::Compose)
                .after(promote_shown_occupancy)
                .after(promote_shown_cover),
        );
    }
}

type RosterRow = (Entity, DrawnCell, StatBlockData);

fn handle_battle_roster(
    reads: ShownBattleReads,
    rows: Query<RosterRow>,
    mounts: Query<&Mounted>,
    mut queue: ResMut<PendingQueue<CommandCall<BattleRoster>>>,
) {
    if queue.is_empty() {
        return;
    }
    let shown = reads.shown();
    let player = shown.player().map(|faction| **faction);
    let mut gangers: Vec<GangerCardNet> = Vec::new();
    for (entity, cell, row) in &rows {
        let faction = *row.faction;
        let visible = shown.ganger_visible(cell.at(), faction).is_squad_visible();
        if player != Some(faction) && !visible {
            continue;
        }
        gangers.push(ganger_card(
            entity,
            CellLevelNet::from_sim(cell.at()),
            &row,
            mounts.get(entity).ok(),
        ));
    }
    gangers.sort_unstable_by_key(|card| *card.token);

    for (_args, responder) in take_calls::<BattleRoster>(&mut queue) {
        responder.answer(&BattleRosterReply {
            gangers: gangers.clone(),
        });
    }
}
