use bevy::prelude::*;
use gdtf_battle_presenter::PresenterSystems;
use gdtf_battle_sim::{
    entity::TerrainCell, ganger::Faction, openable::OpenState, prelude::CellLevel,
};
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_command::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};
use gdtf_qa_protocol::command::{CommandAvailability, CommandName, CommandSummary, CommandTiming};
use serde::{Deserialize, Serialize};

use super::{
    availability::presenter_is_ready,
    shown::{DrawnCell, ShownBattleReads},
};
use crate::{
    dev::net_qa::{
        facts::GameFacts,
        wire::{
            cell::CellLevelNet,
            inspect::CoverBlockNet,
            token::{DoorToken, GangerToken},
            visible::{DoorOpenNet, VisibleCoverNet, VisibleDoorNet, VisibleGangerNet},
        },
    },
    states::running::game::battlescape::inspect_panel::{
        decide::{ShownBattle, object_entry},
        shadow::{promote_shown_cover, promote_shown_emplacements, promote_shown_occupancy},
    },
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BattleVisibleArgs {}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct BattleVisibleReply {
    enemies: Vec<VisibleGangerNet>,
    doors:   Vec<VisibleDoorNet>,
    cover:   Vec<VisibleCoverNet>,
}

pub(crate) struct BattleVisible;

impl QaCommand for BattleVisible {
    type Args = BattleVisibleArgs;
    type Facts = GameFacts;
    type Parked = ();
    type Reply = BattleVisibleReply;

    const NAME: CommandName = CommandName::from_static("battle.visible");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Read the enemies, doors and cover inside the lit area — nothing the fog is hiding. It \
         reads the playback-gated fog and reports each enemy on the cell its sprite is drawn \
         on, so it agrees with what is on screen rather than with the sim ahead of it.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &GameFacts) -> CommandAvailability {
        presenter_is_ready(*facts)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_battle_visible
                .after(QaCommandSystems::Claim)
                .after(PresenterSystems::Compose)
                .after(promote_shown_occupancy)
                .after(promote_shown_cover)
                .after(promote_shown_emplacements),
        );
    }
}

type DoorRow = (Entity, &'static OpenState, &'static TerrainCell);

type GangerRow = (Entity, DrawnCell, &'static Faction);

fn handle_battle_visible(
    reads: ShownBattleReads,
    gangers: Query<GangerRow>,
    doors: Query<DoorRow>,
    mut queue: ResMut<PendingQueue<CommandCall<BattleVisible>>>,
) {
    if queue.is_empty() {
        return;
    }
    let shown = reads.shown();
    let reply = BattleVisibleReply {
        enemies: visible_enemies(shown, &gangers),
        doors:   visible_doors(shown, &doors),
        cover:   visible_cover(shown),
    };
    for (_args, responder) in take_calls::<BattleVisible>(&mut queue) {
        responder.answer(&reply);
    }
}

fn visible_enemies(shown: ShownBattle<'_>, gangers: &Query<GangerRow>) -> Vec<VisibleGangerNet> {
    let player = shown.player().map(|faction| **faction);
    let mut enemies: Vec<VisibleGangerNet> = gangers
        .iter()
        .filter_map(|(entity, cell, faction)| {
            let faction = *faction;
            if player == Some(faction) {
                return None;
            }
            let at = cell.at();
            shown
                .ganger_visible(at, faction)
                .is_squad_visible()
                .then(|| {
                    VisibleGangerNet::new(
                        GangerToken::new(entity.to_bits()),
                        CellLevelNet::from_sim(at),
                    )
                })
        })
        .collect();
    enemies.sort_unstable_by_key(|enemy| *enemy.token);
    enemies
}

fn visible_doors(shown: ShownBattle<'_>, doors: &Query<DoorRow>) -> Vec<VisibleDoorNet> {
    let mut open_doors: Vec<VisibleDoorNet> = doors
        .iter()
        .filter(|(_, _, at)| shown.cell_visible(***at).is_squad_visible())
        .map(|(entity, state, at)| {
            VisibleDoorNet::new(
                DoorToken::new(entity.to_bits()),
                CellLevelNet::from_sim(**at),
                DoorOpenNet::new(*state.is_open()),
            )
        })
        .collect();
    open_doors.sort_unstable_by_key(|door| *door.token);
    open_doors
}

fn visible_cover(shown: ShownBattle<'_>) -> Vec<VisibleCoverNet> {
    let Some(fog) = shown.fog() else {
        return Vec::new();
    };
    let mut cells: Vec<CellLevel> = fog.visible_cells().copied().collect();
    cells.sort_unstable_by_key(|at| (at.level(), at.cell().y, at.cell().x));
    cells
        .into_iter()
        .filter_map(|at| {
            // Terrain the cover ledger has no entry for carries no block to draw, so it is left out.
            let entry = object_entry(at, shown)?.cover()?;
            Some(VisibleCoverNet::new(
                CellLevelNet::from_sim(at),
                CoverBlockNet::from_sim(entry),
            ))
        })
        .collect()
}
