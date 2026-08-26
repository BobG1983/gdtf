use super::assert_ron_round_trip;
use crate::dev::net_qa::wire::{
    cell::{CellLevelNet, CellNet, CellXNet, CellYNet, LevelNet},
    reachable::ReachableCellNet,
    vitals::TuNet,
};

fn a_cell() -> CellLevelNet {
    CellLevelNet::new(
        CellNet::new(CellXNet::new(3), CellYNet::new(-7)),
        LevelNet::new(2),
    )
}

#[test]
fn a_reached_cell_round_trips_with_its_cost() {
    assert_ron_round_trip(&ReachableCellNet::new(a_cell(), TuNet::new(0)));
    assert_ron_round_trip(&ReachableCellNet::new(a_cell(), TuNet::new(17)));
}

#[test]
fn a_reached_cell_refuses_an_unknown_field() {
    let hostile = "(at:(cell:(x:0,y:0),level:0),cost:4,extra:2)";
    assert!(
        ron::de::from_str::<ReachableCellNet>(hostile).is_err(),
        "`{hostile}` carries an unknown field and must not decode",
    );
}
