//! The pure `AoE` template resolver — enumerate the affected `(cell, level)` set a
//! shot's [`HitType`] template covers at its impact cell (GTW-541).

use bevy::math::Vec2;

use crate::{
    metric::{Cell, CellLevel, Level, cell_center},
    occupancy::{GRID_HEIGHT, GRID_WIDTH},
    weapon::{AoeRange, BlastRadius, ConeHalfAngle, HitType},
};

/// Whether a ground-plane `(x, y)` lies inside the coarse grid's
/// [`GRID_WIDTH`] × [`GRID_HEIGHT`] bounds — the edge-clamp guard so a template
/// centred near an edge never emits an off-grid cell (GTW-541 edge case).
fn in_bounds(x: i32, y: i32) -> bool {
    x >= 0
        && y >= 0
        && usize::try_from(x).is_ok_and(|x| x < GRID_WIDTH)
        && usize::try_from(y).is_ok_and(|y| y < GRID_HEIGHT)
}

/// A cell's ground-plane centre as a 2D sim-unit [`Vec2`] — the `(x, y)` of its
/// [`cell_center`] on the given storey. Reuses the metric's cell-centre helper (which
/// owns the int→float conversion) so this module does no raw `as f32` cast; the storey
/// is irrelevant to a ground-plane bearing, so any [`Level`] works.
fn ground_centre(cell: Cell, level: Level) -> Vec2 {
    let c = cell_center(cell, level);
    Vec2::new(c.x, c.y)
}

/// Enumerate the set of affected `(cell, level)` a shot's `hit` template covers at its
/// `impact` cell, given the `shooter` origin (needed for the directional cone / line
/// shapes) — the pure GTW-541 `AoE` resolver.
///
/// Returns a CANONICALLY SORTED `Vec<CellLevel>` (`(z, y, x)` order — the sim's
/// `cell_key` order) so the caller resolves struck entities in a deterministic order and
/// the seeded RNG stream stays byte-stable (GTW-541 determinism AC). Takes NO RNG draw —
/// the geometry is a pure function of `(impact, hit, shooter)`.
///
/// Every emitted cell is on the `impact`'s own storey (the 2D-on-level ruling above) and
/// is clamped to the grid bounds (`in_bounds`); an off-grid candidate is dropped. The
/// `impact` cell is ALWAYS in the returned set (even for a zero-radius / zero-range
/// template), so the direct impact-cell occupant is never missed. For
/// [`HitType::Single`] the set is exactly `[impact]` (the caller short-circuits to the
/// single-target path for `Single`, so this arm exists only for completeness / testing).
///
/// - **Blast{radius}** — the same-storey Chebyshev disc of `radius` around `impact`.
/// - **Cone{range, angle}** — every cell within `range` (Chebyshev) of `impact` whose
///   bearing from the `shooter` is within `angle` degrees of the shooter→impact direction,
///   plus `impact` itself. A degenerate shooter==impact (point-blank) direction falls back
///   to the full disc (no meaningful direction to wedge along).
/// - **Line{range}** — `impact` plus up to `range` cells stepping the shooter→impact
///   ground direction (the discrete [`Direction`](crate::ganger::Direction) cell step). A
///   degenerate shooter==impact direction yields just `[impact]` (no line to draw).
#[must_use]
pub fn aoe_affected(impact: CellLevel, hit: HitType, shooter: CellLevel) -> Vec<CellLevel> {
    // The canonical CellLevel accessors (GTW-565): the impact's (cell, level) and
    // the shooter's ground cell.
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

    // Clamp to grid bounds, lift each to the impact storey, dedup, and sort canonically
    // ((z, y, x) — the sim cell_key order) so the resolution order is deterministic.
    let mut out: Vec<CellLevel> = cells
        .into_iter()
        .filter(|c| in_bounds(c.x, c.y))
        .map(|c| CellLevel::new(c, level))
        .collect();
    out.sort_by_key(|cl| (cl.z, cl.y, cl.x));
    out.dedup();
    out
}

/// The same-storey Chebyshev disc of `radius` cells around `centre` (a
/// [`HitType::Blast`] template). Radius `0` = just `centre`.
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

/// The [`HitType::Line`] beam — `impact` plus up to `range` cells stepping the
/// shooter→impact ground [`Direction`](crate::ganger::Direction). A degenerate
/// shooter==impact direction yields just `[impact]`.
fn line_cells(impact: Cell, shooter: Cell, range: AoeRange) -> Vec<Cell> {
    let mut cells = vec![impact];
    let Some(dir) = crate::ganger::Direction::from_cells(shooter, impact) else {
        return cells; // point-blank: no direction, no line
    };
    let step = dir.cell_step();
    let mut cursor = impact;
    for _ in 0..*range {
        cursor = Cell::new(cursor.x + step.x, cursor.y + step.y);
        cells.push(cursor);
    }
    cells
}

/// The [`HitType::Cone`] wedge — cells within `range` (Chebyshev) of `impact` whose
/// bearing from `shooter` is within `angle` degrees of the shooter→impact direction,
/// plus `impact` itself. `level` is the impact storey (the ground-plane bearing is
/// storey-independent, but `ground_centre` needs a [`Level`]).
///
/// The wedge apex is the SHOOTER (the fire direction is shooter→impact); a candidate cell
/// is inside the wedge when the angle between (candidate − shooter) and (impact − shooter)
/// is ≤ `angle`. A degenerate shooter==impact (point-blank) has no meaningful fire
/// direction, so it falls back to the full `blast_cells` disc of `range` around `impact`.
fn cone_cells(
    impact: Cell,
    shooter: Cell,
    level: Level,
    range: AoeRange,
    angle: ConeHalfAngle,
) -> Vec<Cell> {
    let shooter_c = ground_centre(shooter, level);
    let impact_c = ground_centre(impact, level);
    // The fire direction shooter→impact as a ground-plane vector.
    let fire = impact_c - shooter_c;
    // Point-blank (shooter == impact): no direction to wedge along → the full disc. The
    // cone's `range` becomes the disc radius (the doc's "full disc of `range`"): a directed
    // reach reused as an omnidirectional radius, same cell count.
    if fire.length_squared() <= f32::EPSILON {
        return blast_cells(impact, BlastRadius::new(*range));
    }
    let cos_half = (*angle).to_radians().cos();

    let r = i32::from(*range);
    let mut cells = vec![impact];
    for dy in -r..=r {
        for dx in -r..=r {
            if dx == 0 && dy == 0 {
                continue; // impact already included
            }
            let cell = Cell::new(impact.x + dx, impact.y + dy);
            // The candidate's bearing from the shooter.
            let cand = ground_centre(cell, level) - shooter_c;
            if cand.length_squared() <= f32::EPSILON {
                continue; // the shooter's own cell has no bearing — skip
            }
            // cos(θ) = dot / (|fire| · |cand|); inside the wedge iff θ ≤ angle, i.e.
            // cos(θ) ≥ cos(angle). `Vec2::angle_between` avoids the manual normalize.
            let cos_theta = fire.dot(cand) / (fire.length() * cand.length());
            if cos_theta >= cos_half {
                cells.push(cell);
            }
        }
    }
    cells
}
