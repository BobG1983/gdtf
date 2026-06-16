//! The DATA-DRIVEN cyclic-act order (GTW-225 / GTW-48 S8 AC8): the fixed authored
//! wrap order for the cyclic acts, DEFINED here so 222b / 222c step it from one
//! source rather than inventing an ad-hoc inline `match` that could diverge.
//!
//! The orders are coordinate-system FACTS (the cardinal compass; the posture
//! ladder), not tuning magnitudes — so they live as authored `const` arrays read
//! by [`next_in_cycle`], the single "step-and-wrap" helper. 222b consumes these to
//! build the [`SetStanceRequested`](gdtf_battle_sim::acts::SetStanceRequested) /
//! [`SetFacingRequested`](gdtf_battle_sim::acts::SetFacingRequested) it writes to
//! the intent seam.

use gdtf_battle_sim::{Direction, StanceKind};

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

#[cfg(test)]
mod tests {
    use super::*;

    /// AC8 — facing-cycle steps the 8 directions in the authored order and wraps
    /// `NorthWest → North`. Asserting the exact next-of each pins the FACT.
    #[test]
    fn facing_cycle_steps_the_authored_compass_order_with_wrap() {
        assert_eq!(next_facing(Direction::North), Direction::NorthEast);
        assert_eq!(next_facing(Direction::NorthEast), Direction::East);
        assert_eq!(next_facing(Direction::East), Direction::SouthEast);
        assert_eq!(next_facing(Direction::SouthEast), Direction::South);
        assert_eq!(next_facing(Direction::South), Direction::SouthWest);
        assert_eq!(next_facing(Direction::SouthWest), Direction::West);
        assert_eq!(next_facing(Direction::West), Direction::NorthWest);
        // The wrap.
        assert_eq!(next_facing(Direction::NorthWest), Direction::North);
    }

    /// AC8 — stance-cycle steps the 3 postures and wraps `Prone → Standing`.
    #[test]
    fn stance_cycle_steps_the_posture_ladder_with_wrap() {
        assert_eq!(next_stance(StanceKind::Standing), StanceKind::Crouching);
        assert_eq!(next_stance(StanceKind::Crouching), StanceKind::Prone);
        // The wrap.
        assert_eq!(next_stance(StanceKind::Prone), StanceKind::Standing);
    }

    /// The authored orders enumerate the full closed sets (8 facings, 3 stances) —
    /// so stepping is total and a full lap returns to the start.
    #[test]
    fn cycles_enumerate_the_full_sets_and_lap_back() {
        assert_eq!(FACING_CYCLE.len(), 8, "all 8 Directions are in the cycle");
        assert_eq!(STANCE_CYCLE.len(), 3, "all 3 StanceKinds are in the cycle");

        // A full lap of the facing cycle returns to North.
        let mut dir = Direction::North;
        for _ in 0..FACING_CYCLE.len() {
            dir = next_facing(dir);
        }
        assert_eq!(dir, Direction::North, "a full facing lap returns to North");

        // A full lap of the stance cycle returns to Standing.
        let mut stance = StanceKind::Standing;
        for _ in 0..STANCE_CYCLE.len() {
            stance = next_stance(stance);
        }
        assert_eq!(
            stance,
            StanceKind::Standing,
            "a full stance lap returns to Standing",
        );
    }
}
