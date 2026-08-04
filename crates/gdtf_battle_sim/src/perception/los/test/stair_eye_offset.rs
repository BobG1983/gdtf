//! acceptance tests — the authored stair-tile eye-offset for height-aware LOS.
//!   authored stair tile sees over a [`HeightBand::Mid`] cover cell that a ground-level
use super::support::*;
use crate::{los::probe::eye_anchor, occupancy::StairEyeOffset};

#[test]
fn stair_kneel_clears_mid_cover_ground_kneel_blocks() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();

    let mut cover = CoverLedger::new();
    cover.insert(key(3, 5, 0), cover_entry(HeightBand::Mid));

    let obs_pos = position(0, 5, 0);
    let obs_stance = stance(StanceKind::Crouching);
    let obs_facing = facing(Direction::East);
    let tgt_pos = position(6, 5, 0);
    let tgt_stance = stance(StanceKind::Crouching);
    let target = Target {
        position: &tgt_pos,
        stance:   &tgt_stance,
    };

    let plain_occupancy = OccupancyGrid::new();
    let ground_kneel = Observer {
        position:         &obs_pos,
        stance:           &obs_stance,
        facing:           &obs_facing,
        stair_eye_offset: StairEyeOffset::new(0.0),
        peek_offset:      PeekOffset::default(),
    };
    let ground_sighted = has_los(
        &ground_kneel,
        &target,
        &plain_occupancy,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );
    assert!(
        !*ground_sighted,
        "a ground-kneel observer (no stair) must be BLOCKED by a MID cover \
         (the control geometry that makes the stair effect measurable)"
    );

    let mut stair_occupancy = OccupancyGrid::new();
    stair_occupancy.mark_stair_cell(key(0, 5, 0));
    let stair_offset = stair_occupancy.stair_eye_offset_at(&key(0, 5, 0));
    let stair_kneel = Observer {
        position:         &obs_pos,
        stance:           &obs_stance,
        facing:           &obs_facing,
        stair_eye_offset: stair_offset,
        peek_offset:      PeekOffset::default(),
    };
    let stair_sighted = has_los(
        &stair_kneel,
        &target,
        &stair_occupancy,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );
    assert!(
        *stair_sighted,
        "a kneeling observer on an authored stair tile must SEE OVER a MID cover \
         (+0.5 eye-lift clears the band the ground-kneel was blocked by)"
    );
}

#[test]
fn stair_facing_invariant() {
    let tuning = CombatTuning::default();
    let pos = position(4, 4, 0);
    let st = stance(StanceKind::Crouching);

    let stair_off = StairEyeOffset::new(0.5);

    let north_facing = facing(Direction::North);
    let reference = eye_anchor(
        &Observer {
            position:         &pos,
            stance:           &st,
            facing:           &north_facing,
            stair_eye_offset: stair_off,
            peek_offset:      PeekOffset::default(),
        },
        &tuning,
    );

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
        let f = facing(dir);
        let eye = eye_anchor(
            &Observer {
                position:         &pos,
                stance:           &st,
                facing:           &f,
                stair_eye_offset: stair_off,
                peek_offset:      PeekOffset::default(),
            },
            &tuning,
        );
        assert_eq!(
            (eye.x.to_bits(), eye.y.to_bits(), eye.z.to_bits()),
            (
                reference.x.to_bits(),
                reference.y.to_bits(),
                reference.z.to_bits()
            ),
            "rotating facing to {dir:?} on a stair cell must NOT move the \
             facing-neutral stair eye anchor (facing-invariant)"
        );
    }
}

#[test]
fn non_stair_cell_yields_zero_offset() {
    let tuning = CombatTuning::default();
    let plain = OccupancyGrid::new();

    let queried_cell = key(5, 5, 0);
    let offset = plain.stair_eye_offset_at(&queried_cell);
    assert_eq!(
        (*offset).to_bits(),
        0.0_f32.to_bits(),
        "a plain OccupancyGrid with no stair cells must return StairEyeOffset(0.0) \
         for every cell"
    );

    let pos = position(5, 5, 0);
    let st = stance(StanceKind::Standing);
    let f = facing(Direction::East);
    let baseline = eye_anchor(
        &Observer {
            position:         &pos,
            stance:           &st,
            facing:           &f,
            stair_eye_offset: StairEyeOffset::new(0.0),
            peek_offset:      PeekOffset::default(),
        },
        &tuning,
    );
    let from_grid = eye_anchor(
        &Observer {
            position:         &pos,
            stance:           &st,
            facing:           &f,
            stair_eye_offset: offset,
            peek_offset:      PeekOffset::default(),
        },
        &tuning,
    );
    assert_eq!(
        (
            baseline.x.to_bits(),
            baseline.y.to_bits(),
            baseline.z.to_bits()
        ),
        (
            from_grid.x.to_bits(),
            from_grid.y.to_bits(),
            from_grid.z.to_bits()
        ),
        "a non-stair cell's StairEyeOffset(0.0) must yield a bit-identical eye \
         anchor to an explicit zero-offset (the non-stair path is a no-op)"
    );
}

#[test]
fn prone_stair_no_lift() {
    let tuning = CombatTuning::default();
    let pos = position(3, 3, 0);
    let prone_st = stance(StanceKind::Prone);
    let f = facing(Direction::East);

    let baseline = eye_anchor(
        &Observer {
            position:         &pos,
            stance:           &prone_st,
            facing:           &f,
            stair_eye_offset: StairEyeOffset::new(0.0),
            peek_offset:      PeekOffset::default(),
        },
        &tuning,
    );
    let stair = eye_anchor(
        &Observer {
            position:         &pos,
            stance:           &prone_st,
            facing:           &f,
            stair_eye_offset: StairEyeOffset::new(0.5),
            peek_offset:      PeekOffset::default(),
        },
        &tuning,
    );
    assert_eq!(
        baseline.z.to_bits(),
        stair.z.to_bits(),
        "a Prone observer on a stair cell must get ZERO eye-lift \
         (stance gate: Prone → 0.0, the stair offset is suppressed)"
    );
}
