//! The `AoE` **hit-template resolver** — the pure, render-free function that enumerates
//! the set of affected `(cell, level)` a shot's [`HitType`](crate::weapon::HitType)
//! template covers at its impact cell (GTW-541, CORE of GTW-41).
//!
//! `docs/combat/combat.md` (the one-line blast-radii note — "blast radii ... assume
//! square tiles") is the only `docs/` anchor for area weapons on the square grid; the
//! template GEOMETRY below (Chebyshev disc, cone half-angle wedge, line beam, 2D-on-level,
//! flat falloff) is defined HERE, not in `docs/`. After the `resolution.md` §2 ray-march
//! resolves an impact cell, an `AoE` weapon does not strike
//! just that cell's occupant — it applies its template (a blast disc, a cone wedge, or a
//! line beam) and strikes EVERY occupant in the covered set. This module owns the
//! GEOMETRY of those templates as a pure function over the sim's cell metric; the live
//! fire path ([`fire`](crate::fire::fire)) reads the returned set, queries each cell's
//! [`occupant`](crate::occupancy::OccupancyGrid::occupant), and routes each struck entity
//! through the EXISTING damage pipeline (it reimplements no damage math).
//!
//! ## The templates (GTW-541)
//!
//! - [`HitType::Single`](crate::weapon::HitType::Single) — the impact cell only (the
//!   unchanged pre-GTW-541 single-target path; the live fire path never even calls this
//!   resolver for `Single`, but the resolver returns the singleton set for completeness).
//! - [`HitType::Blast`](crate::weapon::HitType::Blast)`{ radius }` — an
//!   **omnidirectional** disc: every cell within `radius` (Chebyshev — the square grid's
//!   natural king-move distance) of the impact cell, **on the impact's own storey**.
//!   Radius `0` = the impact cell only.
//! - [`HitType::Cone`](crate::weapon::HitType::Cone)`{ range, angle }` — a **directed
//!   wedge** whose apex is the SHOOTER: every cell within `range` cells (Chebyshev) of the
//!   impact whose bearing from the shooter is within `angle` degrees of the shooter→impact
//!   fire direction (plus the impact cell itself), on the impact's storey.
//! - [`HitType::Line`](crate::weapon::HitType::Line)`{ range }` — a **beam**: the impact
//!   cell plus up to `range` cells stepping the shooter→impact ground direction, on the
//!   impact's storey.
//!
//! ## Design rulings (GTW-541 — logged, defensible defaults)
//!
//! - **Blast is 2D-on-level, not a 3D ball.** The coarse model bands VERTICAL exposure by
//!   silhouette height (§1/§2), and a square-grid blast template is a HORIZONTAL footprint
//!   (`docs/combat/combat.md`: "blast radii ... assume square tiles"). So the disc lies on
//!   the impact's own storey; a multi-storey spherical blast is a later tuning axis, not
//!   this CORE. The same 2D-on-level rule applies to the cone wedge and the line beam.
//! - **FLAT damage across the whole set — no distance falloff.** Every affected cell takes
//!   the full shot's damage (the logged GTW-541 default); a falloff curve is a later ticket.
//! - **Determinism.** The returned set is SORTED canonically (`(z, y, x)` — the sim's
//!   `cell_key` order), so the live path resolves struck entities in a stable order and the
//!   seeded RNG stream is byte-stable across runs. This function itself takes NO RNG draw.
//! - **Friendly fire hits ALL occupants** — the resolver enumerates CELLS, faction-blind;
//!   the fire path strikes every occupant it finds (`docs/combat/resolution.md` §2: "any
//!   other actor in the path — including your own gang — true friendly fire"). Grenades do
//!   not discriminate. The set INCLUDES the shooter's own cell if the geometry covers it.
//!
//! Pure model logic: no systems, no `&mut World`, no ECS trigger, no pixel. Every cell is
//! clamped to the [`GRID_WIDTH`](crate::occupancy::GRID_WIDTH) ×
//! [`GRID_HEIGHT`](crate::occupancy::GRID_HEIGHT) bounds so an edge-impact template never
//! yields an off-grid cell (edge-clamp, GTW-541 edge case).

use bevy::math::Vec2;

use crate::{
    metric::{Cell, CellLevel, Level, cell_center},
    occupancy::{GRID_HEIGHT, GRID_WIDTH},
    weapon::HitType,
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
    let level = Level::new(u8::try_from(impact.z).unwrap_or(0));
    let impact_cell = Cell::new(impact.x, impact.y);
    let shooter_cell = Cell::new(shooter.x, shooter.y);

    let cells: Vec<Cell> = match hit {
        HitType::Single => vec![impact_cell],
        HitType::Blast { radius } => blast_cells(impact_cell, *radius),
        HitType::Cone { range, angle } => {
            cone_cells(impact_cell, shooter_cell, level, *range, *angle)
        }
        HitType::Line { range } => line_cells(impact_cell, shooter_cell, *range),
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
fn blast_cells(centre: Cell, radius: u8) -> Vec<Cell> {
    let r = i32::from(radius);
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
fn line_cells(impact: Cell, shooter: Cell, range: u8) -> Vec<Cell> {
    let mut cells = vec![impact];
    let Some(dir) = crate::ganger::Direction::from_cells(shooter, impact) else {
        return cells; // point-blank: no direction, no line
    };
    let step = dir.cell_step();
    let mut cursor = impact;
    for _ in 0..range {
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
fn cone_cells(impact: Cell, shooter: Cell, level: Level, range: u8, angle: f32) -> Vec<Cell> {
    let shooter_c = ground_centre(shooter, level);
    let impact_c = ground_centre(impact, level);
    // The fire direction shooter→impact as a ground-plane vector.
    let fire = impact_c - shooter_c;
    // Point-blank (shooter == impact): no direction to wedge along → the full disc.
    if fire.length_squared() <= f32::EPSILON {
        return blast_cells(impact, range);
    }
    let cos_half = angle.to_radians().cos();

    let r = i32::from(range);
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

#[cfg(test)]
mod tests {
    use super::aoe_affected;
    use crate::{
        metric::{Cell, CellLevel, Level},
        weapon::{AoeRange, BlastRadius, ConeHalfAngle, HitType},
    };

    /// A ground-floor `(cell, level)` key at `(x, y)`.
    fn ground(x: i32, y: i32) -> CellLevel {
        CellLevel::new(Cell::new(x, y), Level::new(0))
    }

    /// The set as a sorted `(x, y)` list on the impact storey (level ignored — every
    /// emitted cell is on the impact's own storey by the 2D-on-level ruling).
    fn xy(set: &[CellLevel]) -> Vec<(i32, i32)> {
        set.iter().map(|c| (c.x, c.y)).collect()
    }

    #[test]
    fn single_is_the_impact_cell_only() {
        let set = aoe_affected(ground(5, 5), HitType::Single, ground(3, 5));
        assert_eq!(xy(&set), vec![(5, 5)], "Single covers only the impact cell");
    }

    #[test]
    fn blast_radius_zero_is_the_impact_cell_only() {
        let set = aoe_affected(
            ground(5, 5),
            HitType::Blast {
                radius: BlastRadius::new(0),
            },
            ground(3, 5),
        );
        assert_eq!(
            xy(&set),
            vec![(5, 5)],
            "radius-0 blast = the impact cell only"
        );
    }

    #[test]
    fn blast_radius_one_is_the_moore_9_disc() {
        let set = aoe_affected(
            ground(5, 5),
            HitType::Blast {
                radius: BlastRadius::new(1),
            },
            ground(3, 5),
        );
        // The Chebyshev disc of radius 1 = the impact + its Moore-8 ring = 9 cells.
        let mut expected: Vec<(i32, i32)> = Vec::new();
        for y in 4..=6 {
            for x in 4..=6 {
                expected.push((x, y));
            }
        }
        assert_eq!(
            xy(&set),
            expected,
            "radius-1 blast is the 3x3 Chebyshev disc"
        );
    }

    #[test]
    fn blast_is_sorted_canonically_and_all_on_the_impact_storey() {
        let impact = CellLevel::new(Cell::new(10, 10), Level::new(3));
        let set = aoe_affected(
            impact,
            HitType::Blast {
                radius: BlastRadius::new(1),
            },
            CellLevel::new(Cell::new(8, 10), Level::new(3)),
        );
        // Every emitted cell is on the impact storey (2D-on-level).
        assert!(
            set.iter().all(|c| c.z == 3),
            "every blast cell lies on the impact's own storey",
        );
        // The set is sorted (z, y, x) — a deterministic resolution order.
        let mut sorted = set.clone();
        sorted.sort_by_key(|c| (c.z, c.y, c.x));
        assert_eq!(
            set, sorted,
            "the affected set is canonically sorted (z, y, x)"
        );
    }

    #[test]
    fn blast_at_the_grid_corner_clamps_off_grid_cells() {
        // Impact at the (0,0) corner; a radius-1 blast's off-grid neighbours are dropped.
        let set = aoe_affected(
            ground(0, 0),
            HitType::Blast {
                radius: BlastRadius::new(1),
            },
            ground(2, 0),
        );
        // Only the in-bounds quadrant survives: (0,0),(1,0),(0,1),(1,1).
        assert_eq!(
            xy(&set),
            vec![(0, 0), (1, 0), (0, 1), (1, 1)],
            "an edge blast clamps its off-grid cells",
        );
    }

    #[test]
    fn line_runs_range_cells_in_the_fire_direction() {
        // Shooter west of the impact → the fire direction is East; the line runs +x.
        let set = aoe_affected(
            ground(5, 5),
            HitType::Line {
                range: AoeRange::new(2),
            },
            ground(2, 5),
        );
        assert_eq!(
            xy(&set),
            // Sorted (y, x): (5,5),(6,5),(7,5) all at y=5.
            vec![(5, 5), (6, 5), (7, 5)],
            "a range-2 line east = the impact + 2 cells further east",
        );
    }

    #[test]
    fn line_range_zero_is_the_impact_cell_only() {
        let set = aoe_affected(
            ground(5, 5),
            HitType::Line {
                range: AoeRange::new(0),
            },
            ground(2, 5),
        );
        assert_eq!(
            xy(&set),
            vec![(5, 5)],
            "a range-0 line = the impact cell only"
        );
    }

    #[test]
    fn line_at_the_edge_clamps() {
        // Impact at the right edge (x=59) firing further east → the off-grid cells drop.
        let set = aoe_affected(
            ground(59, 5),
            HitType::Line {
                range: AoeRange::new(3),
            },
            ground(57, 5),
        );
        assert_eq!(
            xy(&set),
            vec![(59, 5)],
            "a line at the far edge keeps only the in-bounds impact cell",
        );
    }

    #[test]
    fn cone_covers_a_wedge_toward_the_impact_and_excludes_the_flanks() {
        // Shooter at (5,5) firing East; impact at (8,5). A 30 deg half-angle wedge of
        // range 2 covers the forward cells but NOT a cell far off-axis.
        let set = aoe_affected(
            ground(8, 5),
            HitType::Cone {
                range: AoeRange::new(2),
                angle: ConeHalfAngle::new(30.0),
            },
            ground(5, 5),
        );
        let cells = xy(&set);
        // The impact is always included.
        assert!(cells.contains(&(8, 5)), "the cone includes the impact cell");
        // A cell directly ahead (further east, still near the axis) is inside the wedge.
        assert!(
            cells.contains(&(9, 5)) || cells.contains(&(10, 5)),
            "the cone reaches forward along the fire axis: {cells:?}",
        );
        // A cell sharply off the axis (behind/beside the impact, large bearing angle) is
        // OUTSIDE a narrow 30 deg wedge — e.g. two cells north of the impact.
        assert!(
            !cells.contains(&(8, 3)),
            "a sharply off-axis cell is outside the narrow wedge: {cells:?}",
        );
    }

    #[test]
    fn a_wide_cone_covers_more_than_a_narrow_one() {
        let narrow = aoe_affected(
            ground(8, 5),
            HitType::Cone {
                range: AoeRange::new(2),
                angle: ConeHalfAngle::new(15.0),
            },
            ground(5, 5),
        );
        let wide = aoe_affected(
            ground(8, 5),
            HitType::Cone {
                range: AoeRange::new(2),
                angle: ConeHalfAngle::new(80.0),
            },
            ground(5, 5),
        );
        assert!(
            wide.len() > narrow.len(),
            "a wider half-angle wedge covers strictly more cells ({} > {})",
            wide.len(),
            narrow.len(),
        );
    }

    #[test]
    fn point_blank_cone_falls_back_to_the_full_disc() {
        // Shooter == impact (no fire direction): the cone degrades to the full disc of
        // `range` around the impact (the documented degenerate fallback).
        let cone = aoe_affected(
            ground(5, 5),
            HitType::Cone {
                range: AoeRange::new(1),
                angle: ConeHalfAngle::new(30.0),
            },
            ground(5, 5),
        );
        let disc = aoe_affected(
            ground(5, 5),
            HitType::Blast {
                radius: BlastRadius::new(1),
            },
            ground(5, 5),
        );
        assert_eq!(
            cone, disc,
            "a point-blank cone (shooter == impact) falls back to the full radius disc",
        );
    }

    #[test]
    fn the_resolver_takes_no_rng_and_is_a_pure_function() {
        // Two identical calls yield an identical set (a pure function — the determinism
        // property the live path relies on to keep the RNG stream stable).
        let a = aoe_affected(
            ground(5, 5),
            HitType::Blast {
                radius: BlastRadius::new(2),
            },
            ground(3, 5),
        );
        let b = aoe_affected(
            ground(5, 5),
            HitType::Blast {
                radius: BlastRadius::new(2),
            },
            ground(3, 5),
        );
        assert_eq!(a, b, "the resolver is a pure function of its inputs");
    }
}
