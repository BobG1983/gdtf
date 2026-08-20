//! Path preview target and population from selected shooter to goal cell.

use bevy::{
    ecs::{query::QueryData, system::SystemParam},
    prelude::*,
};
use gdtf_battle_presenter::PathPreview;
use gdtf_battle_sim::{
    acts::{DismountSurcharge, dismount_surcharge, move_tu_cost, seat_departure},
    emplacement::{EmplacementEntrySides, EmplacementFacing, Mounted},
    entity::TerrainCell,
    floor::FloorCostGrid,
    injuries::{InflictedInjuries, MovementCostFactor},
    pathfinder::{Departure, MoveGrids, PlanningView, find_path_leaving},
    prelude::{CellLevel, Faction, OccupancyGrid, Position},
    tuning::CombatTuning,
    vertical::VerticalLinkGraph,
    visibility::{FactionRelation, SquadVisibility},
};

use crate::{fire_mode::ChosenFireMode, selection::resources::SelectedShooter};

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

/// Clear the path preview target when a gun's chosen fire mode changes.
pub fn reset_move_target_on_fire_mode_change(
    mode_changed: Query<(), Changed<ChosenFireMode>>,
    mut target: ResMut<PathPreviewTarget>,
) {
    if mode_changed.iter().next().is_none() {
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

/// What a preview reads off the ganger whose route it is drawing.
#[derive(QueryData)]
pub struct PreviewMoverRow {
    position: &'static Position,
    faction:  &'static Faction,
    injuries: Option<&'static InflictedInjuries>,
    mounted:  Option<&'static Mounted>,
}

/// What a preview reads off the seat a mounted ganger rides.
type PreviewSeatRow = (
    &'static TerrainCell,
    Option<&'static EmplacementEntrySides>,
    Option<&'static EmplacementFacing>,
);

/// Grid resources and seat rows needed to plan a path preview.
#[derive(SystemParam)]
pub struct PreviewGrids<'w, 's> {
    grid:        Res<'w, OccupancyGrid>,
    links:       Res<'w, VerticalLinkGraph>,
    squad:       Res<'w, SquadVisibility>,
    tuning:      Res<'w, CombatTuning>,
    floor_costs: Res<'w, FloorCostGrid>,
    seats:       Query<'w, 's, PreviewSeatRow>,
}

impl PreviewGrids<'_, '_> {
    /// Where a route off `from` may leave: the seat's entry cells while the mover rides one.
    fn departure(&self, from: CellLevel, mounted: Option<&Mounted>) -> Departure {
        let Some(seat) = mounted.and_then(Mounted::emplacement) else {
            return Departure::anywhere(from);
        };
        let Ok((cell, sides, facing)) = self.seats.get(seat) else {
            return Departure::anywhere(from);
        };
        seat_departure(from, **cell, sides, facing)
    }
}

/// What one preview is planned from, once the selection and the target both resolve.
struct PreviewPlan {
    departure: Departure,
    goal:      CellLevel,
    faction:   Faction,
    factor:    MovementCostFactor,
    surcharge: DismountSurcharge,
}

/// Recompute the path preview from selected shooter to target cell.
pub fn populate_path_preview(
    selected: Res<SelectedShooter>,
    target: Res<PathPreviewTarget>,
    actors: Query<PreviewMoverRow>,
    factions: Query<&'static Faction>,
    grids: PreviewGrids,
    mut preview: ResMut<PathPreview>,
) {
    let next = match resolve_inputs(*selected, *target, &actors, &grids) {
        Some(plan) => route_for(&plan, &factions, &grids),
        None => PathPreview::cleared(),
    };

    if *preview != next {
        *preview = next;
    }
}

fn resolve_inputs(
    selected: SelectedShooter,
    target: PathPreviewTarget,
    actors: &Query<PreviewMoverRow>,
    grids: &PreviewGrids,
) -> Option<PreviewPlan> {
    let entity = (*selected)?;
    let goal = (*target)?;
    let row = actors.get(entity).ok()?;
    let factor = row.injuries.map_or(
        MovementCostFactor::IDENTITY,
        InflictedInjuries::movement_cost_factor,
    );
    Some(PreviewPlan {
        departure: grids.departure(**row.position, row.mounted),
        goal,
        faction: *row.faction,
        factor,
        surcharge: dismount_surcharge(row.mounted, &grids.tuning),
    })
}

fn route_for(
    plan: &PreviewPlan,
    factions: &Query<&'static Faction>,
    grids: &PreviewGrids,
) -> PathPreview {
    let mover_faction = plan.faction;
    let planning = PlanningView::new(&grids.squad, |occupant| {
        relation_to(factions, mover_faction, occupant)
    });
    match find_path_leaving(
        &plan.departure,
        plan.goal,
        MoveGrids {
            occupancy:   &grids.grid,
            links:       &grids.links,
            floor_costs: &grids.floor_costs,
            tuning:      &grids.tuning,
        },
        plan.factor,
        &planning,
    ) {
        Ok(path) => PathPreview::new(path.cells().to_vec(), move_tu_cost(&path, plan.surcharge)),
        Err(_blocked) => PathPreview::cleared(),
    }
}
