//! Pure-value helper tests — `authored_cells` union and stacked-ganger detection,
//! no spawn or world.

use bevy::platform::collections::HashSet;

// PROOF MIGRATION (GTW-324): this concern file constructs its gangers/situations via
// the crate-central `test_support` builders directly, validating the canonical
// builder end-to-end from inside the sim's own unit tests.
use crate::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    metric::CellLevel,
    occupancy::TerrainKind,
    situation::{CoverSpawn, has_stacked_gangers},
    test_support::{SituationBuilder, ganger_at, key},
};

/// `authored_cells` is the union of wall, scatter, and slab cells — NOT ganger
/// cells (a ganger does not author a tile a link can attach to).
#[test]
fn authored_cells_unions_walls_scatter_slabs_only() {
    let wall = key(1, 1, 0);
    let prop = key(2, 2, 0);
    let slab = key(3, 3, 1);
    let ganger = key(9, 9, 0);
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(ganger, 0))
        .wall_at(wall)
        .with_scatter(CoverSpawn::new(
            prop,
            TerrainKind::Cover,
            CoverHp::new(20),
            HeightBand::Low,
            ArmorProtection::new(1),
            ArmorHardness::new(0),
        ))
        .slab_at(slab)
        .build();

    let cells: HashSet<CellLevel> = situation.authored_cells().collect();
    assert!(cells.contains(&wall), "wall cell is authored");
    assert!(cells.contains(&prop), "scatter cell is authored");
    assert!(cells.contains(&slab), "slab cell is authored");
    assert!(
        !cells.contains(&ganger),
        "a ganger cell does NOT author a tile",
    );
    assert_eq!(cells.len(), 3, "exactly the three terrain/slab cells");
}

/// `has_stacked_gangers` detects two gangers on one `(cell, level)` and is false
/// for distinct cells.
#[test]
fn stacked_ganger_detection() {
    let at = key(5, 5, 0);
    // Built via the canonical [`SituationBuilder`] (GTW-324) — two gangers at the SAME cell
    // (the builder does not dedupe, preserving the stack the detection under test reads).
    let stacked = SituationBuilder::new()
        .with_gangers(vec![ganger_at(at, 0), ganger_at(at, 1)])
        .build();
    assert!(
        has_stacked_gangers(&stacked),
        "two gangers on one cell stack"
    );

    // The central two-ganger-plus-terrain fixture has distinct ganger cells.
    let clean = crate::test_support::fixtures::minimal_with_cells();
    assert!(
        !has_stacked_gangers(&clean),
        "distinct ganger cells do not stack",
    );
}
