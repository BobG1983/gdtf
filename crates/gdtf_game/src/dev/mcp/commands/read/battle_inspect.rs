use bevy::prelude::*;
use cobalt_mcp_command::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming,
};
use cobalt_mcp_transport::PendingQueue;
use gdtf_battle_presenter::PresenterSystems;
use gdtf_battle_sim::emplacement::Mounted;
use serde::{Deserialize, Serialize};

use super::{
    availability::presenter_is_ready,
    shown::{ShownBattleReads, ganger_card},
};
use crate::{
    dev::mcp::{
        facts::GameFacts,
        wire::{
            cell::CellLevelNet,
            inspect::{
                CoverBlockNet, EmplacementStateNet, InspectShownNet, InspectTerrainNet,
                MountedWeaponNet, TerrainKindNet,
            },
        },
    },
    states::running::game::battlescape::{
        inspect_panel::{
            decide::{InspectShown, InspectTerrain, inspect_shown},
            shadow::{promote_shown_cover, promote_shown_emplacements, promote_shown_occupancy},
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
        "Read one cell exactly as the inspect panel would draw it: the card for a \
         squad-visible ganger standing there and the terrain half for the cell itself — its \
         piece kind, the cover stats the ledger holds, and an emplacement's state and mounted \
         weapon. Either half is absent when there is nothing to report, and a cell the squad \
         cannot see reports neither. Takes the cell to read; the panel's own hovered and \
         pinned cells come from battle.selection.",
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
                .after(promote_shown_cover)
                .after(promote_shown_emplacements),
        );
    }
}

fn handle_battle_inspect(
    reads: ShownBattleReads,
    rows: Query<StatBlockData>,
    mounts: Query<&Mounted>,
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
            shown: on_the_wire(&decision, args.at, &rows, &mounts),
        });
    }
}

fn on_the_wire(
    decision: &InspectShown,
    at: CellLevelNet,
    rows: &Query<StatBlockData>,
    mounts: &Query<&Mounted>,
) -> InspectShownNet {
    InspectShownNet {
        ganger:  decision.ganger().and_then(|entity| {
            rows.get(entity)
                .ok()
                .map(|row| ganger_card(entity, at, &row, mounts.get(entity).ok()))
        }),
        terrain: decision.terrain().map(terrain_on_the_wire),
    }
}

fn terrain_on_the_wire(terrain: &InspectTerrain) -> InspectTerrainNet {
    let seat = terrain.emplacement();
    InspectTerrainNet {
        kind:   TerrainKindNet::from_sim(terrain.kind()),
        cover:  terrain.cover().map(CoverBlockNet::from_sim),
        state:  seat.map(|seat| EmplacementStateNet::from_sim(seat.state())),
        weapon: seat.map(|seat| MountedWeaponNet::new((***seat.weapon()).clone())),
    }
}
