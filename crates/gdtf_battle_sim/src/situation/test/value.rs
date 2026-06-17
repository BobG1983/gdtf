//! Pure-value helper tests — `authored_cells` union and stacked-ganger detection,
//! no spawn or world.

use bevy::platform::collections::HashSet;

use super::support::*;

/// `authored_cells` is the union of wall, scatter, and slab cells — NOT ganger
/// cells (a ganger does not author a tile a link can attach to).
#[test]
fn authored_cells_unions_walls_scatter_slabs_only() {
    let wall = key(1, 1, 0);
    let prop = key(2, 2, 0);
    let slab = key(3, 3, 1);
    let ganger = key(9, 9, 0);
    let situation = Situation {
        gangers: vec![ganger_at(ganger, 0)],
        walls: vec![wall_at(wall)],
        scatter: vec![CoverSpawn::new(
            prop,
            TerrainKind::Cover,
            CoverHp::new(20),
            HeightBand::Low,
            ArmorProtection::new(1),
            ArmorHardness::new(0),
        )],
        slabs: vec![slab],
        ..Situation::new()
    };

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
    let stacked = Situation {
        gangers: vec![ganger_at(at, 0), ganger_at(at, 1)],
        ..Situation::new()
    };
    assert!(
        has_stacked_gangers(&stacked),
        "two gangers on one cell stack"
    );

    let (clean, ..) = minimal_fixture();
    assert!(
        !has_stacked_gangers(&clean),
        "distinct ganger cells do not stack",
    );
}
