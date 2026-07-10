//! `is_8_adjacent` — the same-level Moore-8 reach itself: all 8 ringing cells are
//! adjacent, the centre and a two-cell step are not, and a different storey never
//! is.

use super::support::{GROUND, Level, is_8_adjacent, pos};

/// All 8 surrounding same-level cells are adjacent; the center cell is not; a
/// two-cell step is not.
#[test]
fn moore_8_neighbours_are_adjacent_self_and_far_are_not() {
    let center = pos(10, 10, GROUND);
    // The 8 ringing cells (dx, dy in {-1, 0, 1}, not both 0) are adjacent.
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
    // A two-cell step (Chebyshev 2) is out of reach.
    assert!(
        !*is_8_adjacent(center, pos(12, 10, GROUND)),
        "a two-cell step is NOT 8-adjacent",
    );
    assert!(
        !*is_8_adjacent(center, pos(12, 12, GROUND)),
        "a two-cell diagonal step is NOT 8-adjacent",
    );
}

/// A neighbour on a DIFFERENT storey (same x/y, z off by one) is NOT adjacent —
/// reach is the 8 same-level cells, not the 26 cross-level voxels.
#[test]
fn a_different_storey_is_not_adjacent() {
    let a = pos(5, 5, Level::new(2));
    // Directly above: same cell, one storey up.
    assert!(
        !*is_8_adjacent(a, pos(5, 5, Level::new(3))),
        "the cell directly above is NOT 8-adjacent (same-level only)",
    );
    // A would-be Moore neighbour but a storey up — still not adjacent.
    assert!(
        !*is_8_adjacent(a, pos(6, 5, Level::new(3))),
        "a Moore neighbour on another storey is NOT 8-adjacent",
    );
}
