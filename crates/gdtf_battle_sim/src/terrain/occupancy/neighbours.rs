//! Planar pathable neighbours with diagonal corner-cutting and move-cost scaling.

use std::f32::consts::SQRT_2;

use bevy::prelude::Deref;

use crate::{
    ganger::Tu,
    injuries::MovementCostFactor,
    metric::{Cell, CellLevel},
    occupancy::OccupancyGrid,
    terrain::floor::FloorCostGrid,
    tuning::MoveCost,
};

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
struct Diagonal(bool);

impl Diagonal {
    const fn new(diagonal: bool) -> Self {
        Self(diagonal)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
struct CornerCut(bool);

impl CornerCut {
    const fn new(cut: bool) -> Self {
        Self(cut)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
struct CellDelta(i32);

impl CellDelta {
    const fn new(step: i32) -> Self {
        Self(step)
    }
}

const PLANAR_OFFSETS: [(i32, i32); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

/// Eight-way neighbours that are in bounds, not path-blocked, and do not cut corners.
pub fn pathable_neighbors<'a>(
    origin: CellLevel,
    grid: &'a OccupancyGrid,
    floor_costs: &'a FloorCostGrid,
    factor: MovementCostFactor,
) -> impl Iterator<Item = (CellLevel, Tu)> + 'a {
    PLANAR_OFFSETS.into_iter().filter_map(move |(dx, dy)| {
        let neighbour = planar_neighbour(origin, CellDelta::new(dx), CellDelta::new(dy));
        if grid.slot(&neighbour).is_none() || *grid.is_path_blocked(&neighbour) {
            return None;
        }
        let diagonal = dx != 0 && dy != 0;
        if diagonal && *corner_is_cut(origin, CellDelta::new(dx), CellDelta::new(dy), grid) {
            return None;
        }
        let base = step_cost(floor_costs.cost(&neighbour), Diagonal::new(diagonal));
        let cost = scale_by_factor(base, factor);
        Some((neighbour, cost))
    })
}

fn planar_neighbour(origin: CellLevel, dx: CellDelta, dy: CellDelta) -> CellLevel {
    CellLevel::new(Cell::new(origin.x + *dx, origin.y + *dy), origin.level())
}

fn corner_is_cut(
    origin: CellLevel,
    dx: CellDelta,
    dy: CellDelta,
    grid: &OccupancyGrid,
) -> CornerCut {
    let side_a = planar_neighbour(origin, dx, CellDelta::new(0));
    let side_b = planar_neighbour(origin, CellDelta::new(0), dy);
    CornerCut::new(*grid.is_path_blocked(&side_a) && *grid.is_path_blocked(&side_b))
}

fn step_cost(floor_cost: MoveCost, diagonal: Diagonal) -> Tu {
    let orthogonal = *floor_cost;
    if !*diagonal {
        return Tu::new(orthogonal);
    }
    let octile = (f32::from(orthogonal) * SQRT_2).round() as u8;
    Tu::new(octile)
}

fn scale_by_factor(base: Tu, factor: MovementCostFactor) -> Tu {
    if factor == MovementCostFactor::IDENTITY {
        return base;
    }
    let scaled = {
        let raw = (f32::from(*base) * factor.raw()).ceil();
        if raw > f32::from(u8::MAX) {
            u8::MAX
        } else {
            raw as u8
        }
    };
    Tu::new(scaled)
}
