//! The authored cyclic orders + the single step-and-wrap helper.

use gdtf_battle_sim::prelude::{Direction, StanceKind};

/// The authored facing-cycle order — the 8 [`Direction`]s in cardinal-compass order
/// with wrap (`North` → `NorthEast` → ... → `NorthWest` → `North`).
///
/// A fixed FACT (the square grid's clockwise compass), read by [`next_facing`] —
/// not an ad-hoc `match`. The array IS the order; stepping it is wrapping
/// next-of-index.
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

/// The authored stance-cycle order — the 3 [`StanceKind`]s in the posture ladder
/// with wrap (Standing → Crouching → Prone → Standing).
///
/// A fixed FACT (the upright→kneel→flat ladder), read by [`next_stance`] — not an
/// ad-hoc `match`.
pub const STANCE_CYCLE: [StanceKind; 3] = [
    StanceKind::Standing,
    StanceKind::Crouching,
    StanceKind::Prone,
];

/// The element AFTER `current` in `order`, wrapping past the end back to the start.
///
/// The single step-and-wrap primitive both [`next_facing`] and [`next_stance`] use,
/// generic over the authored order array. An element NOT found in `order`
/// (impossible for the closed `Direction` / `StanceKind` sets, which the const
/// arrays enumerate in full) wraps to the FIRST element — a total, never-panicking
/// fallback (no `unwrap` on the position lookup). The empty-array case also returns
/// `current` unchanged (defensive; the shipped arrays are non-empty).
fn next_in_cycle<T: Copy + PartialEq>(order: &[T], current: T) -> T {
    let Some(index) = order.iter().position(|&item| item == current) else {
        // Not in the authored order — restart at the first element, or echo
        // `current` for a (never-shipped) empty order.
        return order.first().copied().unwrap_or(current);
    };
    let next = (index + 1) % order.len();
    // `next` is `index + 1` mod a non-empty length, so it is always in range; the
    // `unwrap_or` only guards the impossible empty-array case.
    order.get(next).copied().unwrap_or(current)
}

/// The [`Direction`] after `current` in the authored [`FACING_CYCLE`] order, wrapping
/// `NorthWest → North`.
///
/// The single facing-step used by 222b's facing-cycle act — read from the authored
/// order, never inlined.
#[must_use]
pub fn next_facing(current: Direction) -> Direction {
    next_in_cycle(&FACING_CYCLE, current)
}

/// The [`StanceKind`] after `current` in the authored [`STANCE_CYCLE`] order, wrapping
/// `Prone → Standing`.
///
/// The single stance-step used by 222b's stance-cycle act — read from the authored
/// order, never inlined.
#[must_use]
pub fn next_stance(current: StanceKind) -> StanceKind {
    next_in_cycle(&STANCE_CYCLE, current)
}
