//! The [`Direction`] compass: `forward_step` unit-vectors and the 8-way ring
//! helpers (`steps_to`, `from_cells`, `rotated_toward`).

use crate::{ganger::Direction, metric::Cell};

// --- GTW-168 AC #1: Direction::forward_step() — each of the 8 variants maps to
// a documented ground-plane unit step in sim units, sign + axis matching the
// named direction, and every step normalised (length 1 within f32 tolerance).
// No pixel, no screen-coordinate convention.

/// A loose f32 tolerance for the unit-length / component checks — the diagonal
/// `1/√2` components are not exactly representable, so an exact compare is wrong.
const STEP_TOL: f32 = 1.0e-6;

#[test]
fn forward_step_signs_and_axes_match_each_direction() {
    // The diagonal per-axis magnitude (positive); a diagonal spans two axes at
    // equal magnitude, the cardinals one axis at magnitude 1.
    let d = core::f32::consts::FRAC_1_SQRT_2;

    // (direction, expected x, expected y) — z is always 0 (ground plane).
    let cases = [
        (Direction::North, 0.0, -1.0),
        (Direction::NorthEast, d, -d),
        (Direction::East, 1.0, 0.0),
        (Direction::SouthEast, d, d),
        (Direction::South, 0.0, 1.0),
        (Direction::SouthWest, -d, d),
        (Direction::West, -1.0, 0.0),
        (Direction::NorthWest, -d, -d),
    ];

    for (dir, ex, ey) in cases {
        let step = dir.forward_step();
        assert!(
            (step.x - ex).abs() < STEP_TOL,
            "{dir:?}: x {} should match {ex}",
            step.x,
        );
        assert!(
            (step.y - ey).abs() < STEP_TOL,
            "{dir:?}: y {} should match {ey}",
            step.y,
        );
        // The step lies in the ground plane — z is exactly zero ("up" is the
        // separate +z axis).
        assert_eq!(step.z.to_bits(), 0.0_f32.to_bits(), "{dir:?}: z must be 0");
    }
}

#[test]
fn forward_step_is_unit_length_for_every_direction() {
    for dir in [
        Direction::North,
        Direction::NorthEast,
        Direction::East,
        Direction::SouthEast,
        Direction::South,
        Direction::SouthWest,
        Direction::West,
        Direction::NorthWest,
    ] {
        let len = dir.forward_step().length();
        assert!(
            (len - 1.0).abs() < STEP_TOL,
            "{dir:?}: forward_step must be unit length, got {len}",
        );
    }
}

#[test]
fn forward_step_diagonals_have_equal_axis_magnitude() {
    // A diagonal's two non-zero axes share one magnitude (so it points exactly
    // 45° between its two cardinals), distinguishing it from a longer (±1, ±1).
    for dir in [
        Direction::NorthEast,
        Direction::SouthEast,
        Direction::SouthWest,
        Direction::NorthWest,
    ] {
        let step = dir.forward_step();
        assert!(
            (step.x.abs() - step.y.abs()).abs() < STEP_TOL,
            "{dir:?}: diagonal axes must share magnitude: {} vs {}",
            step.x.abs(),
            step.y.abs(),
        );
    }
}

// --- GTW-235: the 8-way ring helpers. The fixed clockwise ring (the same
// order the variants are declared in): North, NE, E, SE, S, SW, W, NW.
const RING: [Direction; 8] = [
    Direction::North,
    Direction::NorthEast,
    Direction::East,
    Direction::SouthEast,
    Direction::South,
    Direction::SouthWest,
    Direction::West,
    Direction::NorthWest,
];

// GTW-235 AC1 — steps_to is the short-way 45deg count: for every ordered pair
// a.steps_to(b) == min(d, 8-d) with d = |ord(a)-ord(b)|; 0 iff a == b; symmetric;
// exactly 4 for every opposite pair; never exceeds 4.

#[test]
fn steps_to_is_the_short_way_45deg_count_for_every_pair() {
    for (ia, &a) in RING.iter().enumerate() {
        for (ib, &b) in RING.iter().enumerate() {
            // The ordinal gap, computed independently of the helper (the spec form).
            let ia = u8::try_from(ia).unwrap_or(0);
            let ib = u8::try_from(ib).unwrap_or(0);
            let d = ia.abs_diff(ib);
            let expected = d.min(8 - d);

            let got = a.steps_to(b);
            assert_eq!(got, expected, "{a:?}.steps_to({b:?}) must be min(d, 8-d)");
            // 0 iff equal.
            assert_eq!(got == 0, a == b, "{a:?}.steps_to({b:?}) is 0 iff equal");
            // Symmetric.
            assert_eq!(got, b.steps_to(a), "steps_to must be symmetric");
            // Never exceeds 4 (half the ring).
            assert!(got <= 4, "{a:?}.steps_to({b:?}) = {got} must not exceed 4");
        }
    }

    // Every opposite pair is exactly 4 (the four diameters of the ring).
    let opposites = [
        (Direction::North, Direction::South),
        (Direction::NorthEast, Direction::SouthWest),
        (Direction::East, Direction::West),
        (Direction::SouthEast, Direction::NorthWest),
    ];
    for (a, b) in opposites {
        assert_eq!(a.steps_to(b), 4, "{a:?} and {b:?} are opposite (4 steps)");
    }
}

// GTW-235 AC2 — from_cells gives the compass dir toward the target. Smaller y is
// North (the forward_step −Y convention); coincident cells give None.

#[test]
fn from_cells_points_the_8_way_compass_toward_the_target() {
    let origin = Cell::new(5, 5);
    // The full 8-way table around (5,5): a neighbour in each compass direction.
    let cases = [
        (Cell::new(8, 5), Direction::East),      // dx>0, dy==0
        (Cell::new(5, 2), Direction::North),     // dx==0, dy<0 (smaller y is North)
        (Cell::new(8, 2), Direction::NorthEast), // dx>0, dy<0
        (Cell::new(2, 8), Direction::SouthWest), // dx<0, dy>0
        (Cell::new(2, 5), Direction::West),      // dx<0, dy==0
        (Cell::new(5, 8), Direction::South),     // dx==0, dy>0
        (Cell::new(8, 8), Direction::SouthEast), // dx>0, dy>0
        (Cell::new(2, 2), Direction::NorthWest), // dx<0, dy<0
    ];
    for (to, expected) in cases {
        assert_eq!(
            Direction::from_cells(origin, to),
            Some(expected),
            "from_cells({origin:?}, {to:?}) must point {expected:?}",
        );
    }

    // Coincident cells: no direction toward yourself.
    assert_eq!(
        Direction::from_cells(origin, origin),
        None,
        "from_cells of coincident cells must be None",
    );
    // Distance does not matter, only the sign pair (a far East cell is still East).
    assert_eq!(
        Direction::from_cells(Cell::new(0, 0), Cell::new(40, 0)),
        Some(Direction::East),
        "from_cells reads only the per-axis sign, not the magnitude",
    );
}

// GTW-235 AC3 — rotated_toward advances the short way, clamps (no overshoot), breaks
// the opposite-facing tie clockwise, and is consistent with steps_to.

#[test]
fn rotated_toward_advances_the_short_way_clamped_with_clockwise_tie() {
    // Clockwise short way (North -> East is +2 clockwise).
    assert_eq!(
        Direction::North.rotated_toward(Direction::East, 1),
        Direction::NorthEast,
        "one step North toward East is NorthEast",
    );
    assert_eq!(
        Direction::North.rotated_toward(Direction::East, 2),
        Direction::East,
        "two steps North toward East reach East",
    );
    // Clamp: more steps than needed never overshoots the target.
    assert_eq!(
        Direction::North.rotated_toward(Direction::East, 9),
        Direction::East,
        "rotated_toward clamps — it never overshoots the target",
    );

    // Opposite facing (North <-> South, cw == ccw == 4): the tie breaks CLOCKWISE.
    assert_eq!(
        Direction::North.rotated_toward(Direction::South, 1),
        Direction::NorthEast,
        "the opposite-facing tie breaks clockwise (one step is NorthEast)",
    );
    assert_eq!(
        Direction::North.rotated_toward(Direction::South, 2),
        Direction::East,
        "two clockwise steps from North toward South reach East",
    );
    assert_eq!(
        Direction::North.rotated_toward(Direction::South, 4),
        Direction::South,
        "four steps from North reach the opposite South",
    );

    // Counter-clockwise short way (North -> West is -2 counter-clockwise).
    assert_eq!(
        Direction::North.rotated_toward(Direction::West, 1),
        Direction::NorthWest,
        "one step North toward West (the short way) is NorthWest",
    );

    // Identity: rotating toward self, or zero steps, stays put — for ALL facings.
    for &d in &RING {
        assert_eq!(d.rotated_toward(d, 3), d, "{d:?} toward itself stays put");
        for &t in &RING {
            assert_eq!(
                d.rotated_toward(t, 0),
                d,
                "{d:?}.rotated_toward({t:?}, 0) must stay put",
            );
            // Consistency: rotating the full short-way count reaches the target.
            assert_eq!(
                d.rotated_toward(t, d.steps_to(t)),
                t,
                "{d:?}.rotated_toward({t:?}, steps_to) must reach {t:?}",
            );
        }
    }
}
