//! Fire-target highlight for a fireable hover cell.

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_presenter::{FireTargetHighlight, cell_squad_visible};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    ganger::{Aiming, TuMax},
    magazine::mode_tu_cost,
    prelude::{CellLevel, Faction, OccupancyGrid, Tu},
    tuning::CombatTuning,
    visibility::{FactionRelation, SquadVisibility},
};

use crate::{InspectTarget, SelectedFireMode, selection::resources::SelectedShooter};

/// Resources read when resolving the fire-target highlight.
#[derive(SystemParam)]
pub struct FireTargetReads<'w> {
    selected: Res<'w, SelectedShooter>,
    fire_mode: Res<'w, SelectedFireMode>,
    inspect: Res<'w, InspectTarget>,
    occupancy: Res<'w, OccupancyGrid>,
    player: Res<'w, PlayerFaction>,
    tuning: Res<'w, CombatTuning>,
    squad: Option<Res<'w, SquadVisibility>>,
}

/// Update the fire-target highlight from hover and selection.
pub fn populate_fire_target(
    reads: FireTargetReads,
    factions: Query<&Faction>,
    shooters: Query<(&TuMax, &Aiming)>,
    mut highlight: ResMut<FireTargetHighlight>,
) {
    let next = resolve_fire_target(&reads, &factions, &shooters);

    if *highlight != next {
        *highlight = next;
    }
}

fn resolve_fire_target(
    reads: &FireTargetReads,
    factions: &Query<&Faction>,
    shooters: &Query<(&TuMax, &Aiming)>,
) -> FireTargetHighlight {
    let player: Faction = **reads.player;
    let Some(cell) = reads.inspect.hovered() else {
        return FireTargetHighlight::cleared();
    };
    let Some(shooter) = **reads.selected else {
        return FireTargetHighlight::cleared();
    };
    let selection_is_player = factions.get(shooter).copied().is_ok_and(|f| f == player);
    if !selection_is_player {
        return FireTargetHighlight::cleared();
    }
    if !cell_is_fireable_target(reads, factions, &cell, player) {
        return FireTargetHighlight::cleared();
    }
    match fire_cost(shooter, &reads.fire_mode, &reads.tuning, shooters) {
        Some(cost) => FireTargetHighlight::new(cell, cost),
        None => FireTargetHighlight::cleared(),
    }
}

fn cell_is_fireable_target(
    reads: &FireTargetReads,
    factions: &Query<&Faction>,
    cell: &CellLevel,
    player: Faction,
) -> bool {
    match reads.occupancy.occupant(cell) {
        Some(occupant) => {
            let is_enemy = factions
                .get(occupant)
                .copied()
                .is_ok_and(|faction| faction != player);
            is_enemy
                && cell_squad_visible(reads.squad.as_deref(), cell, Some(FactionRelation::Other))
                    .is_squad_visible()
        }
        None => {
            *reads.occupancy.is_blocked(cell)
                && cell_squad_visible(reads.squad.as_deref(), cell, None).is_squad_visible()
        }
    }
}

fn fire_cost(
    shooter: Entity,
    fire_mode: &SelectedFireMode,
    tuning: &CombatTuning,
    shooters: &Query<(&TuMax, &Aiming)>,
) -> Option<Tu> {
    let (tu_max, aiming) = shooters.get(shooter).ok()?;
    Some(mode_tu_cost(fire_mode, tu_max, aiming, tuning))
}
