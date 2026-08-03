//! Authored facing and stance cycle orders.

use gdtf_battle_sim::prelude::{Direction, StanceKind};

/// Facing cycle in cardinal-compass order.
pub const FACING_CYCLE: [Direction; 8] = [
    Direction::North,
    Direction::NorthEast,
    Direction::East,
    Direction::SouthEast,
    Direction::South,
    Direction::SouthWest,
    Direction::West,
    Direction::NorthWest,
];

/// Stance cycle: standing → crouching → prone.
pub const STANCE_CYCLE: [StanceKind; 3] = [
    StanceKind::Standing,
    StanceKind::Crouching,
    StanceKind::Prone,
];

fn next_in_cycle<T: Copy + PartialEq>(order: &[T], current: T) -> T {
    let Some(index) = order.iter().position(|&item| item == current) else {
        return order.first().copied().unwrap_or(current);
    };
    let next = (index + 1) % order.len();
    order.get(next).copied().unwrap_or(current)
}

/// Next facing in [`FACING_CYCLE`].
#[must_use]
pub fn next_facing(current: Direction) -> Direction {
    next_in_cycle(&FACING_CYCLE, current)
}

/// Next stance in [`STANCE_CYCLE`].
#[must_use]
pub fn next_stance(current: StanceKind) -> StanceKind {
    next_in_cycle(&STANCE_CYCLE, current)
}
