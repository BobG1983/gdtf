//! Plan an open-door act, or a step toward a reachable closed door.

use bevy::prelude::Entity;

use super::{
    advance::plan_reposition,
    decide::AiTarget,
    params::{AiActRequests, AiPlanningGrids},
    snapshot::GangerRow,
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

pub(super) fn door_rows(
    doors: impl IntoIterator<Item = (Entity, OpenState, CellLevel)>,
) -> Vec<DoorRow> {
    doors
        .into_iter()
        .map(|(entity, state, cell)| DoorRow {
            entity,
            cell,
            state,
        })
        .collect()
}

pub(super) fn write_door_act(orders: &mut AiActRequests, act: DoorAct) {
    match act {
        DoorAct::Open(open) => {
            orders.open_door.write(open);
        }
        DoorAct::Step(step) => {
            orders.step.write(step);
        }
    }
}

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
    let from_cell = from.cell();
    let from_level = from.level();
    pads.iter().copied().min_by_key(|pad| {
        let dx = (from_cell.x - pad.x).unsigned_abs();
        let dy = (from_cell.y - pad.y).unsigned_abs();
        let level_gap = u32::from((*from_level).abs_diff(*pad.level()));
        (dx.max(dy), level_gap, pad.z, pad.y, pad.x)
    })
}
