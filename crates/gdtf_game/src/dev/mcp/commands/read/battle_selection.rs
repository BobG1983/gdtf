use bevy::prelude::*;
use cobalt_mcp_command::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming,
};
use cobalt_mcp_transport::PendingQueue;
use gdtf_battle_input::{FireModeSystems, FiredWeaponModes, InspectTarget, SelectedShooter};
use gdtf_battle_sim::weapon::ModeKind;
use serde::{Deserialize, Serialize};

use super::availability::on_the_battle_screen;
use crate::dev::mcp::{
    facts::GameFacts,
    wire::{cell::CellLevelNet, misc::ModeKindNet, token::GangerToken},
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BattleSelectionArgs {}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct BattleSelectionReply {
    pub(super) shooter:   Option<GangerToken>,
    pub(super) fire_mode: Option<ModeKindNet>,
    pub(super) hovered:   Option<CellLevelNet>,
    pub(super) pinned:    Option<CellLevelNet>,
}

pub(crate) struct BattleSelection;

impl QaCommand for BattleSelection {
    type Args = BattleSelectionArgs;
    type Facts = GameFacts;
    type Parked = ();
    type Reply = BattleSelectionReply;

    const NAME: CommandName = CommandName::from_static("battle.selection");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Read the selected shooter, its fire mode, and the inspect target — the hovered cell \
         and the pinned one, which the panels read with the pin winning. Each field is absent \
         when nothing is selected, hovered or pinned.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &GameFacts) -> CommandAvailability {
        on_the_battle_screen(*facts)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_battle_selection
                .after(QaCommandSystems::Claim)
                .after(FireModeSystems::Write),
        );
    }
}

fn handle_battle_selection(
    selected: Option<Res<SelectedShooter>>,
    arms: FiredWeaponModes,
    inspect: Option<Res<InspectTarget>>,
    mut queue: ResMut<PendingQueue<CommandCall<BattleSelection>>>,
) {
    if queue.is_empty() {
        return;
    }
    let kind = selected
        .as_deref()
        .and_then(|selected| arms.spec_of_selected(*selected))
        .map(|spec| spec.kind);
    let reply = selection_reply(selected.as_deref(), kind, inspect.as_deref());
    for (_args, responder) in take_calls::<BattleSelection>(&mut queue) {
        responder.answer(&reply);
    }
}

/// Read the live selection into the reply that goes out.
pub(super) fn selection_reply(
    selected: Option<&SelectedShooter>,
    fire_mode: Option<ModeKind>,
    inspect: Option<&InspectTarget>,
) -> BattleSelectionReply {
    BattleSelectionReply {
        shooter:   selected
            .and_then(|selected| **selected)
            .map(|entity| GangerToken::new(entity.to_bits())),
        fire_mode: fire_mode.map(ModeKindNet::from_sim),
        hovered:   inspect
            .and_then(InspectTarget::hovered)
            .map(CellLevelNet::from_sim),
        pinned:    inspect
            .and_then(InspectTarget::pinned)
            .map(CellLevelNet::from_sim),
    }
}
