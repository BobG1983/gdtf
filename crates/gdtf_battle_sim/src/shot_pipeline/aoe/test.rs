use super::aoe_affected;
use crate::{
    metric::{Cell, CellLevel, Level},
    weapon::{AoeRange, BlastRadius, ConeHalfAngle, HitType},
};

fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

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
    assert!(
        set.iter().all(|c| c.z == 3),
        "every blast cell lies on the impact's own storey",
    );
    let mut sorted = set.clone();
    sorted.sort_by_key(|c| (c.z, c.y, c.x));
    assert_eq!(
        set, sorted,
        "the affected set is canonically sorted (z, y, x)"
    );
}

#[test]
fn blast_at_the_grid_corner_clamps_off_grid_cells() {
    let set = aoe_affected(
        ground(0, 0),
        HitType::Blast {
            radius: BlastRadius::new(1),
        },
        ground(2, 0),
    );
    assert_eq!(
        xy(&set),
        vec![(0, 0), (1, 0), (0, 1), (1, 1)],
        "an edge blast clamps its off-grid cells",
    );
}

#[test]
fn line_runs_range_cells_in_the_fire_direction() {
    let set = aoe_affected(
        ground(5, 5),
        HitType::Line {
            range: AoeRange::new(2),
        },
        ground(2, 5),
    );
    assert_eq!(
        xy(&set),
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
    let set = aoe_affected(
        ground(8, 5),
        HitType::Cone {
            range: AoeRange::new(2),
            angle: ConeHalfAngle::new(30.0),
        },
        ground(5, 5),
    );
    let cells = xy(&set);
    assert!(cells.contains(&(8, 5)), "the cone includes the impact cell");
    assert!(
        cells.contains(&(9, 5)) || cells.contains(&(10, 5)),
        "the cone reaches forward along the fire axis: {cells:?}",
    );
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
