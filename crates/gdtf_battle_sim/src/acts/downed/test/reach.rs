use super::support::{GROUND, Level, is_8_adjacent, pos};

#[test]
fn moore_8_neighbours_are_adjacent_self_and_far_are_not() {
    let center = pos(10, 10, GROUND);
    for dx in -1..=1 {
        for dy in -1..=1 {
            let other = pos(10 + dx, 10 + dy, GROUND);
            let adjacent = *is_8_adjacent(center, other);
            if dx == 0 && dy == 0 {
                assert!(!adjacent, "the same cell is NOT 8-adjacent (excludes self)");
            } else {
                assert!(
                    adjacent,
                    "({dx},{dy}) is one of the 8 surrounding cells — must be adjacent",
                );
            }
        }
    }
    assert!(
        !*is_8_adjacent(center, pos(12, 10, GROUND)),
        "a two-cell step is NOT 8-adjacent",
    );
    assert!(
        !*is_8_adjacent(center, pos(12, 12, GROUND)),
        "a two-cell diagonal step is NOT 8-adjacent",
    );
}

#[test]
fn a_different_storey_is_not_adjacent() {
    let a = pos(5, 5, Level::new(2));
    assert!(
        !*is_8_adjacent(a, pos(5, 5, Level::new(3))),
        "the cell directly above is NOT 8-adjacent (same-level only)",
    );
    assert!(
        !*is_8_adjacent(a, pos(6, 5, Level::new(3))),
        "a Moore neighbour on another storey is NOT 8-adjacent",
    );
}
