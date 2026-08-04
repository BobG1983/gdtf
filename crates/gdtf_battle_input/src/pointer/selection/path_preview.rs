//! Path preview target and population from selected shooter to goal cell.

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_presenter::PathPreview;
use gdtf_battle_sim::{
    floor::FloorCostGrid,
    injuries::{InflictedInjuries, MovementCostFactor},
    pathfinder::{PlanningView, find_path},
    prelude::{CellLevel, Faction, OccupancyGrid, Position},
    tuning::CombatTuning,
    vertical::VerticalLinkGraph,
    visibility::{FactionRelation, SquadVisibility},
    weapon::FireMode,
};

use crate::{SelectedFireMode, selection::resources::SelectedShooter};

/// Goal cell for the move path preview, if any.
#[derive(Resource, Deref, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PathPreviewTarget(Option<CellLevel>);

impl PathPreviewTarget {
    /// Pin this cell as the move target.
    #[must_use]
    pub const fn new(cell: CellLevel) -> Self {
        Self(Some(cell))
    }

    /// Clear the move target.
    #[must_use]
    pub const fn cleared() -> Self {
        Self(None)
    }
}

/// Clear the path preview target when the fire mode changes (not on select).
pub fn reset_move_target_on_fire_mode_change(
    fire_mode: Res<SelectedFireMode>,
    selected: Res<SelectedShooter>,
    weapon_just_armed: Query<(), Added<FireMode>>,
    mut target: ResMut<PathPreviewTarget>,
) {
    let weapon_arrived = weapon_just_armed.iter().next().is_some();
    if !fire_mode.is_changed() || selected.is_changed() || weapon_arrived {
        return;
    }
    let cleared = PathPreviewTarget::cleared();
    if *target != cleared {
        *target = cleared;
    }
}

fn relation_to(
    factions: &Query<&'static Faction>,
    mover_faction: Faction,
    occupant: Entity,
) -> FactionRelation {
    match factions.get(occupant) {
        Ok(faction) if *faction == mover_faction => FactionRelation::OwnSquad,
        _ => FactionRelation::Other,
    }
}

/// Grid resources needed to plan a path preview.
#[derive(SystemParam)]
pub struct PreviewGrids<'w> {
    grid:        Res<'w, OccupancyGrid>,
    links:       Res<'w, VerticalLinkGraph>,
    squad:       Res<'w, SquadVisibility>,
    tuning:      Res<'w, CombatTuning>,
    floor_costs: Res<'w, FloorCostGrid>,
}

/// Recompute the path preview from selected shooter to target cell.
pub fn populate_path_preview(
    selected: Res<SelectedShooter>,
    target: Res<PathPreviewTarget>,
    actors: Query<(&Position, &Faction, Option<&InflictedInjuries>)>,
    factions: Query<&'static Faction>,
    grids: PreviewGrids,
    mut preview: ResMut<PathPreview>,
) {
    let next = match resolve_inputs(*selected, *target, &actors) {
        Some((start, goal, mover_faction, factor)) => {
            route_for(start, goal, mover_faction, factor, &factions, &grids)
        }
        None => PathPreview::cleared(),
    };

    if *preview != next {
        *preview = next;
    }
}

fn resolve_inputs(
    selected: SelectedShooter,
    target: PathPreviewTarget,
    actors: &Query<(&Position, &Faction, Option<&InflictedInjuries>)>,
) -> Option<(CellLevel, CellLevel, Faction, MovementCostFactor)> {
    let entity = (*selected)?;
    let goal = (*target)?;
    let (position, &mover_faction, injuries) = actors.get(entity).ok()?;
    let factor = injuries.map_or(
        MovementCostFactor::IDENTITY,
        InflictedInjuries::movement_cost_factor,
    );
    Some((**position, goal, mover_faction, factor))
}

fn route_for(
    start: CellLevel,
    goal: CellLevel,
    mover_faction: Faction,
    factor: MovementCostFactor,
    factions: &Query<&'static Faction>,
    grids: &PreviewGrids,
) -> PathPreview {
    let planning = PlanningView::new(&grids.squad, |occupant| {
        relation_to(factions, mover_faction, occupant)
    });
    match find_path(
        start,
        goal,
        &grids.grid,
        &grids.links,
        &grids.tuning,
        &grids.floor_costs,
        factor,
        &planning,
    ) {
        Ok(path) => PathPreview::new(path.cells().to_vec(), path.total()),
        Err(_blocked) => PathPreview::cleared(),
    }
}
