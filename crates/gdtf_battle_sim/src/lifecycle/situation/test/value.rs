//! Pure-value helper tests — `authored_cells` union and stacked-ganger detection,
use bevy::platform::collections::HashSet;

use crate::{
    metric::CellLevel,
    situation::{CoverSpawn, has_stacked_gangers},
    terrain::facing::TerrainFacing,
    test_support::{SituationBuilder, ganger_at, key, test_pieces},
};

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
            test_pieces::COVER,
            TerrainFacing::default(),
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

#[test]
fn stacked_ganger_detection() {
    let at = key(5, 5, 0);
    let stacked = SituationBuilder::new()
        .with_gangers(vec![ganger_at(at, 0), ganger_at(at, 1)])
        .build();
    assert!(
        *has_stacked_gangers(&stacked),
        "two gangers on one cell stack"
    );

    let clean = crate::test_support::fixtures::minimal_with_cells();
    assert!(
        !*has_stacked_gangers(&clean),
        "distinct ganger cells do not stack",
    );
}
