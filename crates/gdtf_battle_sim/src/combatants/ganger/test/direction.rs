use crate::{
    ganger::{Direction, RingSteps},
    metric::Cell,
};

// --- AC #1: Direction::forward_step() — each of the 8 variants maps to

const STEP_TOL: f32 = 1.0e-6;

#[test]
fn forward_step_signs_and_axes_match_each_direction() {
    let d = core::f32::consts::FRAC_1_SQRT_2;

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


#[test]
fn steps_to_is_the_short_way_45deg_count_for_every_pair() {
    for (ia, &a) in RING.iter().enumerate() {
        for (ib, &b) in RING.iter().enumerate() {
            let ia = u8::try_from(ia).unwrap_or(0);
            let ib = u8::try_from(ib).unwrap_or(0);
            let d = ia.abs_diff(ib);
            let expected = d.min(8 - d);

            let got = *a.steps_to(b);
            assert_eq!(got, expected, "{a:?}.steps_to({b:?}) must be min(d, 8-d)");
            assert_eq!(got == 0, a == b, "{a:?}.steps_to({b:?}) is 0 iff equal");
            assert_eq!(got, *b.steps_to(a), "steps_to must be symmetric");
            assert!(got <= 4, "{a:?}.steps_to({b:?}) = {got} must not exceed 4");
        }
    }

    let opposites = [
        (Direction::North, Direction::South),
        (Direction::NorthEast, Direction::SouthWest),
        (Direction::East, Direction::West),
        (Direction::SouthEast, Direction::NorthWest),
    ];
    for (a, b) in opposites {
        assert_eq!(*a.steps_to(b), 4, "{a:?} and {b:?} are opposite (4 steps)");
    }
}


#[test]
fn from_cells_points_the_8_way_compass_toward_the_target() {
    let origin = Cell::new(5, 5);
    let cases = [
        (Cell::new(8, 5), Direction::East),      
        (Cell::new(5, 2), Direction::North),     
        (Cell::new(8, 2), Direction::NorthEast), 
        (Cell::new(2, 8), Direction::SouthWest), 
        (Cell::new(2, 5), Direction::West),      
        (Cell::new(5, 8), Direction::South),     
        (Cell::new(8, 8), Direction::SouthEast), 
        (Cell::new(2, 2), Direction::NorthWest), 
    ];
    for (to, expected) in cases {
        assert_eq!(
            Direction::from_cells(origin, to),
            Some(expected),
            "from_cells({origin:?}, {to:?}) must point {expected:?}",
        );
    }

    assert_eq!(
        Direction::from_cells(origin, origin),
        None,
        "from_cells of coincident cells must be None",
    );
    assert_eq!(
        Direction::from_cells(Cell::new(0, 0), Cell::new(40, 0)),
        Some(Direction::East),
        "from_cells reads only the per-axis sign, not the magnitude",
    );
}


#[test]
fn rotated_toward_advances_the_short_way_clamped_with_clockwise_tie() {
    assert_eq!(
        Direction::North.rotated_toward(Direction::East, RingSteps::new(1)),
        Direction::NorthEast,
        "one step North toward East is NorthEast",
    );
    assert_eq!(
        Direction::North.rotated_toward(Direction::East, RingSteps::new(2)),
        Direction::East,
        "two steps North toward East reach East",
    );
    assert_eq!(
        Direction::North.rotated_toward(Direction::East, RingSteps::new(9)),
        Direction::East,
        "rotated_toward clamps — it never overshoots the target",
    );

    assert_eq!(
        Direction::North.rotated_toward(Direction::South, RingSteps::new(1)),
        Direction::NorthEast,
        "the opposite-facing tie breaks clockwise (one step is NorthEast)",
    );
    assert_eq!(
        Direction::North.rotated_toward(Direction::South, RingSteps::new(2)),
        Direction::East,
        "two clockwise steps from North toward South reach East",
    );
    assert_eq!(
        Direction::North.rotated_toward(Direction::South, RingSteps::new(4)),
        Direction::South,
        "four steps from North reach the opposite South",
    );

    assert_eq!(
        Direction::North.rotated_toward(Direction::West, RingSteps::new(1)),
        Direction::NorthWest,
        "one step North toward West (the short way) is NorthWest",
    );

    for &d in &RING {
        assert_eq!(
            d.rotated_toward(d, RingSteps::new(3)),
            d,
            "{d:?} toward itself stays put"
        );
        for &t in &RING {
            assert_eq!(
                d.rotated_toward(t, RingSteps::new(0)),
                d,
                "{d:?}.rotated_toward({t:?}, 0) must stay put",
            );
            assert_eq!(
                d.rotated_toward(t, d.steps_to(t)),
                t,
                "{d:?}.rotated_toward({t:?}, steps_to) must reach {t:?}",
            );
        }
    }
}
