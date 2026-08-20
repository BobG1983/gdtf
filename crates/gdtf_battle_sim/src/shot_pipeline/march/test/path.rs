use super::support::*;
use crate::march::cells_crossed;

#[test]
fn a_round_crosses_the_cells_between_its_muzzle_and_its_impact() {
    let muzzle = center(4, 4, 0);
    let impact = key(8, 4, 0);
    let path = cells_crossed(muzzle, impact);

    let crossed: Vec<CellLevel> = path.cells().collect();
    assert_eq!(
        crossed.len(),
        5,
        "a round flying four cells along one axis passes through five cells, muzzle cell to \
         impact cell: {crossed:?}",
    );
    assert!(
        path.contains_cell(&key(6, 4, 0)),
        "the cell half way along the flight is neither muzzle nor impact and must still be \
         in the path: {crossed:?}",
    );
    assert_eq!(
        crossed.first().copied(),
        Some(key(4, 4, 0)),
        "the path opens on the cell the round was fired from: {crossed:?}",
    );
    assert_eq!(
        crossed.last().copied(),
        Some(impact),
        "the path ends on the cell the round stopped in: {crossed:?}",
    );
}

#[test]
fn a_round_that_stops_where_it_started_crosses_one_cell() {
    let muzzle = center(3, 3, 0);
    let path = cells_crossed(muzzle, key(3, 3, 0));

    assert_eq!(
        path.len(),
        1,
        "a round that never leaves its own cell crosses exactly that one: {:?}",
        path.cells().collect::<Vec<_>>(),
    );
}
