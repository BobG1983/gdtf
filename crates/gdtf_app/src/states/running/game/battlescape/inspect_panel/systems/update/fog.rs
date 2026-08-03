use bevy::prelude::*;
use gdtf_battle_presenter::cell_squad_visible;
use gdtf_battle_sim::{
    battle::PlayerFaction,
    prelude::{CellLevel, Faction},
    visibility::FactionRelation,
};

use super::params::InspectReads;

pub(super) fn occupant_squad_visible(
    cell: CellLevel,
    occupant: Entity,
    factions: &Query<&Faction>,
    reads: &InspectReads,
) -> bool {
    let relation = occupant_relation(occupant, factions, reads.player.as_deref());
    cell_squad_visible(reads.fog(), &cell, Some(relation)).is_squad_visible()
}

fn occupant_relation(
    occupant: Entity,
    factions: &Query<&Faction>,
    player: Option<&PlayerFaction>,
) -> FactionRelation {
    let occupant_faction = factions.get(occupant).ok().copied();
    match (occupant_faction, player) {
        (Some(faction), Some(player)) if faction == **player => FactionRelation::OwnSquad,
        _ => FactionRelation::Other,
    }
}
