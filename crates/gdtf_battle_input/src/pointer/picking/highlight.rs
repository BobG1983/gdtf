//! Emit cell highlight requests from the inspect target.

use bevy::prelude::*;
use gdtf_battle_presenter::{CellVisibility, HighlightRequest, cell_squad_visible};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    prelude::{Faction, OccupancyGrid},
    visibility::{FactionRelation, SquadVisibility},
};

use crate::picking::hovered::InspectTarget;

/// Write a highlight request for the hovered cell when it is blocked or a visible occupant.
pub fn emit_highlight_request(
    target: Res<InspectTarget>,
    grid: Option<Res<OccupancyGrid>>,
    squad: Option<Res<SquadVisibility>>,
    player: Option<Res<PlayerFaction>>,
    factions: Query<&Faction>,
    mut requests: MessageWriter<HighlightRequest>,
) {
    let highlighted = target.hovered().and_then(|cell| {
        let grid = grid.as_ref()?;
        let relation = grid
            .occupant(&cell)
            .map(|occupant| occupant_relation(occupant, &factions, player.as_deref()));
        let occupant_visible = grid.occupant(&cell).is_some()
            && cell_squad_visible(squad.as_deref(), &cell, relation).is_squad_visible();
        (occupant_visible || *grid.is_blocked(&cell)).then_some(cell)
    });
    let visibility = highlighted.map_or(CellVisibility::NotSquadVisible, |cell| {
        let relation = grid
            .as_ref()
            .and_then(|grid| grid.occupant(&cell))
            .map(|occupant| occupant_relation(occupant, &factions, player.as_deref()));
        cell_squad_visible(squad.as_deref(), &cell, relation)
    });
    requests.write(HighlightRequest::new(highlighted, visibility));
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
