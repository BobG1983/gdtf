use bevy::{math::Vec2, prelude::Deref};

use crate::{
    march::InGrid,
    metric::{Cell, CellLevel, Level, cell_center},
    occupancy::{GRID_HEIGHT, GRID_WIDTH},
    weapon::{AoeRange, BlastRadius, ConeHalfAngle, HitType},
};

#[derive(Deref, Debug, Clone, Copy, PartialEq)]
struct GroundPoint(Vec2);

impl GroundPoint {
        const fn new(point: Vec2) -> Self {
        Self(point)
    }
}

fn in_bounds(cell: Cell) -> InGrid {
    InGrid::new(
        cell.x >= 0
            && cell.y >= 0
            && usize::try_from(cell.x).is_ok_and(|x| x < GRID_WIDTH)
            && usize::try_from(cell.y).is_ok_and(|y| y < GRID_HEIGHT),
    )
}

fn ground_centre(cell: Cell, level: Level) -> GroundPoint {
    let c = cell_center(cell, level);
    GroundPoint::new(Vec2::new(c.x, c.y))
}

#[must_use]
pub fn aoe_affected(impact: CellLevel, hit: HitType, shooter: CellLevel) -> Vec<CellLevel> {
    let (impact_cell, level) = impact.split();
    let shooter_cell = shooter.cell();

    let cells: Vec<Cell> = match hit {
        HitType::Single => vec![impact_cell],
        HitType::Blast { radius } => blast_cells(impact_cell, radius),
        HitType::Cone { range, angle } => {
            cone_cells(impact_cell, shooter_cell, level, range, angle)
        }
        HitType::Line { range } => line_cells(impact_cell, shooter_cell, range),
    };

    let mut out: Vec<CellLevel> = cells
        .into_iter()
        .filter(|c| *in_bounds(*c))
        .map(|c| CellLevel::new(c, level))
        .collect();
    out.sort_by_key(|cl| (cl.z, cl.y, cl.x));
    out.dedup();
    out
}

fn blast_cells(centre: Cell, radius: BlastRadius) -> Vec<Cell> {
    let r = i32::from(*radius);
    let mut cells = Vec::new();
    for dy in -r..=r {
        for dx in -r..=r {
            cells.push(Cell::new(centre.x + dx, centre.y + dy));
        }
    }
    cells
}

fn line_cells(impact: Cell, shooter: Cell, range: AoeRange) -> Vec<Cell> {
    let mut cells = vec![impact];
    let Some(dir) = crate::ganger::Direction::from_cells(shooter, impact) else {
        return cells; 
    };
    let step = dir.cell_step();
    let mut cursor = impact;
    for _ in 0..*range {
        cursor = Cell::new(cursor.x + step.x, cursor.y + step.y);
        cells.push(cursor);
    }
    cells
}

fn cone_cells(
    impact: Cell,
    shooter: Cell,
    level: Level,
    range: AoeRange,
    angle: ConeHalfAngle,
) -> Vec<Cell> {
    let shooter_c = *ground_centre(shooter, level);
    let impact_c = *ground_centre(impact, level);
    let fire = impact_c - shooter_c;
    if fire.length_squared() <= f32::EPSILON {
        return blast_cells(impact, BlastRadius::new(*range));
    }
    let cos_half = (*angle).to_radians().cos();

    let r = i32::from(*range);
    let mut cells = vec![impact];
    for dy in -r..=r {
        for dx in -r..=r {
            if dx == 0 && dy == 0 {
                continue; 
            }
            let cell = Cell::new(impact.x + dx, impact.y + dy);
            let cand = *ground_centre(cell, level) - shooter_c;
            if cand.length_squared() <= f32::EPSILON {
                continue; 
            }
            let cos_theta = fire.dot(cand) / (fire.length() * cand.length());
            if cos_theta >= cos_half {
                cells.push(cell);
            }
        }
    }
    cells
}
