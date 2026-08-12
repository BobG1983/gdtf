//! Fire-target highlight for a fireable hover cell.

use bevy::{
    ecs::{query::QueryData, system::SystemParam},
    prelude::*,
};
use gdtf_battle_presenter::{FireTargetHighlight, cell_squad_visible};
use gdtf_battle_sim::{
    acts::fire_arc_tu_cost,
    battle::PlayerFaction,
    ganger::{Aiming, Facing, TuMax},
    magazine::mode_tu_cost,
    prelude::{CellLevel, Faction, OccupancyGrid, Position, Tu},
    tuning::CombatTuning,
    visibility::{FactionRelation, SquadVisibility},
    weapon::FireModeSpec,
};

use crate::{InspectTarget, fire_mode::FiredWeaponModes, selection::resources::SelectedShooter};

/// Resources read when resolving the fire-target highlight.
#[derive(SystemParam)]
pub struct FireTargetReads<'w, 's> {
    selected:  Res<'w, SelectedShooter>,
    arms:      FiredWeaponModes<'w, 's>,
    inspect:   Res<'w, InspectTarget>,
    occupancy: Res<'w, OccupancyGrid>,
    player:    Res<'w, PlayerFaction>,
    tuning:    Res<'w, CombatTuning>,
    squad:     Option<Res<'w, SquadVisibility>>,
}

/// One shooter's row: what pricing a shot from where it stands needs.
#[derive(QueryData)]
pub struct ShooterPricing {
    tu_max:   &'static TuMax,
    aiming:   &'static Aiming,
    facing:   &'static Facing,
    position: &'static Position,
}

/// Update the fire-target highlight from hover and selection.
pub fn populate_fire_target(
    reads: FireTargetReads,
    factions: Query<&Faction>,
    shooters: Query<ShooterPricing>,
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
    shooters: &Query<ShooterPricing>,
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
    let Some(spec) = reads.arms.spec_of(shooter) else {
        return FireTargetHighlight::cleared();
    };
    match fire_cost(shooter, &cell, spec, reads, shooters) {
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

// The shot plus any turn it needs to face the target.
fn fire_cost(
    shooter: Entity,
    target: &CellLevel,
    spec: FireModeSpec,
    reads: &FireTargetReads,
    shooters: &Query<ShooterPricing>,
) -> Option<Tu> {
    let row = shooters.get(shooter).ok()?;
    let shot = mode_tu_cost(&spec, row.tu_max, row.aiming, &reads.tuning);
    Some(fire_arc_tu_cost(
        **row.facing,
        row.position.cell(),
        target.cell(),
        shot,
        &reads.tuning,
    ))
}
