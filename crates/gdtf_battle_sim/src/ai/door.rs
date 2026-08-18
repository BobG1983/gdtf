//! Plan an open-door act, or a step toward a reachable closed door.

use bevy::prelude::Entity;

use super::{
    advance::plan_reposition,
    decide::{AiTarget, pick_nearest},
    params::AiPlanningGrids,
    snapshot::{GangerRow, row_cell_level},
};
use crate::{
    acts::{
        MoveRequested, OpenDoorRequested, can_open_door,
        downed::is_8_adjacent,
        movement::{BreakAwayMover, suppressed_move_legal},
    },
    ganger::Position,
    metric::{Cell, CellLevel},
    occupancy::OccupancyGrid,
    terrain::openable::OpenState,
    visibility::OmniscientFog,
};

pub(super) struct DoorRow {
    pub(super) entity: Entity,
    pub(super) cell:   CellLevel,
    pub(super) state:  OpenState,
}

pub(super) enum DoorAct {
    Open(OpenDoorRequested),
    Step(MoveRequested),
}

const NEIGHBOUR_XY: [(i32, i32); 8] = [
    (-1, -1),
    (-1, 0),
    (-1, 1),
    (0, -1),
    (0, 1),
    (1, -1),
    (1, 0),
    (1, 1),
];

pub(super) fn plan_door(
    enemy: &GangerRow,
    doors: &[DoorRow],
    rows: &[GangerRow],
    omniscient: Option<&OmniscientFog>,
    grids: &AiPlanningGrids,
    is_dead: &impl Fn(Entity) -> bool,
) -> Option<DoorAct> {
    let mut closed: Vec<&DoorRow> = doors.iter().filter(|door| !*door.state.is_open()).collect();
    closed.sort_by_key(|door| (door.cell.z, door.cell.y, door.cell.x));

    for door in &closed {
        let door_pos = Position::new(door.cell);
        if *can_open_door(
            enemy.position,
            door_pos,
            door.state,
            &enemy.tu,
            grids.tuning(),
        ) {
            return Some(DoorAct::Open(OpenDoorRequested::new(
                enemy.entity,
                door.entity,
            )));
        }
    }

    let omniscient = omniscient?;
    let occupancy = grids.routes().occupancy;
    for door in &closed {
        let door_pos = Position::new(door.cell);
        if *is_8_adjacent(enemy.position, door_pos) {
            continue;
        }
        let pads = walkable_pads(door.cell, occupancy);
        let goal = nearest_pad(enemy.position, &pads)?;
        let dest = plan_reposition(
            enemy,
            &AiTarget::new(door.entity, goal.cell(), goal.level()),
            rows,
            omniscient,
            grids.routes(),
        )?;
        if let Some(suppressor) = enemy.pinned_by {
            let mover =
                BreakAwayMover::new(enemy.entity, &enemy.position, &enemy.stance, &enemy.facing);
            if !*suppressed_move_legal(
                &mover,
                &dest,
                &suppressor,
                grids.cover(),
                &grids.sight(is_dead),
            ) {
                continue;
            }
        }
        return Some(DoorAct::Step(MoveRequested::new(enemy.entity, dest)));
    }
    None
}

fn walkable_pads(door: CellLevel, occupancy: &OccupancyGrid) -> Vec<CellLevel> {
    let mut pads = Vec::new();
    for (dx, dy) in NEIGHBOUR_XY {
        let pad = CellLevel::new(Cell::new(door.x + dx, door.y + dy), door.level());
        if !*occupancy.is_path_blocked(&pad) {
            pads.push(pad);
        }
    }
    pads.sort_by_key(|pad| (pad.z, pad.y, pad.x));
    pads
}

fn nearest_pad(from: Position, pads: &[CellLevel]) -> Option<CellLevel> {
    let from_level = row_cell_level(&from);
    let candidates: Vec<AiTarget> = pads
        .iter()
        .map(|pad| AiTarget::new(Entity::from_bits(0), pad.cell(), pad.level()))
        .collect();
    pick_nearest(from_level.cell(), from_level.level(), &candidates)
        .map(|picked| CellLevel::new(picked.cell, picked.level))
}
