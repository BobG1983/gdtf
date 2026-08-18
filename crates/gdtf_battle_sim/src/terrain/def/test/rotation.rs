use super::super::rotated_entry_sides;
use crate::terrain::facing::TerrainFacing;

#[test]
fn an_authored_side_advances_by_the_steps_to_the_piece_facing() {
    let ring = TerrainFacing::ALL;
    let Some(default_at) = ring
        .iter()
        .position(|cardinal| *cardinal == TerrainFacing::default())
    else {
        unreachable!("TerrainFacing::default() is one of TerrainFacing::ALL");
    };
    let authored = vec![ring[0]];

    for steps in 0..3 {
        let facing = ring[(default_at + steps) % ring.len()];
        assert_eq!(
            rotated_entry_sides(&authored, facing),
            vec![ring[steps]],
            "a piece authored enterable from {:?} and turned {steps} step(s) from the default \
             facing to {facing:?} is entered from {:?}",
            ring[0],
            ring[steps],
        );
    }
}

#[test]
fn a_piece_naming_no_side_rotates_to_no_side() {
    for facing in TerrainFacing::ALL {
        assert!(
            rotated_entry_sides(&[], facing).is_empty(),
            "a def naming no entry side names none under {facing:?} either — a mount authored \
             as scenery stays scenery whichever way it is turned",
        );
    }
}
