use super::support::*;
use crate::{los::probe::eye_anchor, metric::pos_to_cell};

#[test]
fn peek_clears_corner_wall_center_blocks() {
    use bevy::math::Vec2;

    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let mut occupancy = OccupancyGrid::new();

    let mut cover = CoverLedger::new();
    cover.insert(key(3, 5, 0), cover_entry(HeightBand::High));

    let target_entity = spawn_entity();
    place_occupant(
        &mut occupancy,
        key(4, 8, 0),
        target_entity,
        HeightBand::High,
    );

    let obs_pos = position(3, 3, 0);
    let obs_stance = stance(StanceKind::Crouching);
    let obs_facing = facing(Direction::North);
    let tgt_pos = position(4, 8, 0);
    let tgt_stance = stance(StanceKind::Crouching);
    let target = Target {
        position: &tgt_pos,
        stance:   &tgt_stance,
    };

    let centred = Observer {
        position:         &obs_pos,
        stance:           &obs_stance,
        facing:           &obs_facing,
        stair_eye_offset: StairEyeOffset::new(0.0),
        peek_offset:      PeekOffset::default(),
    };
    let centred_sighted = has_los(
        &centred,
        &target,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );
    assert!(
        !*centred_sighted,
        "centred-eye observer must be BLOCKED by the HIGH cover column \
         (the control that makes the peek effect measurable)"
    );

    let peek_sighted = has_los_peeking(
        &centred,
        &target,
        PeekOffset::new(Vec2::new(0.4, 0.0)),
        &occupancy,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );
    assert!(
        *peek_sighted,
        "a peeking observer (eye nudged +0.4 east) must SEE the target past the \
         HIGH cover column — peeked ray crosses cover row at x≈4.08 "
    );

    assert_ne!(
        *centred_sighted, *peek_sighted,
        "peek and no-peek verdicts must DIFFER in the same world (asymmetry)"
    );
}

#[test]
fn clearing_peek_restores_non_peek() {
    let tuning = CombatTuning::default();
    let pos = position(5, 5, 0);
    let st = stance(StanceKind::Standing);
    let f = facing(Direction::East);

    let no_peek = eye_anchor(
        &Observer {
            position:         &pos,
            stance:           &st,
            facing:           &f,
            stair_eye_offset: StairEyeOffset::new(0.0),
            peek_offset:      PeekOffset::default(),
        },
        &tuning,
    );
    let cleared = eye_anchor(
        &Observer {
            position:         &pos,
            stance:           &st,
            facing:           &f,
            stair_eye_offset: StairEyeOffset::new(0.0),
            peek_offset:      PeekOffset::new(bevy::math::Vec2::ZERO),
        },
        &tuning,
    );
    assert_eq!(
        (
            no_peek.x.to_bits(),
            no_peek.y.to_bits(),
            no_peek.z.to_bits()
        ),
        (
            cleared.x.to_bits(),
            cleared.y.to_bits(),
            cleared.z.to_bits()
        ),
        "PeekOffset::new(Vec2::ZERO) and PeekOffset::default() must produce \
         a bit-identical eye anchor (Default is the no-op identity)"
    );
}

#[test]
fn peek_facing_invariant() {
    let tuning = CombatTuning::default();
    let pos = position(4, 4, 0);
    let st = stance(StanceKind::Standing);
    let peek = PeekOffset::new(bevy::math::Vec2::new(0.3, 0.1));

    let north_facing = facing(Direction::North);
    let reference = eye_anchor(
        &Observer {
            position:         &pos,
            stance:           &st,
            facing:           &north_facing,
            stair_eye_offset: StairEyeOffset::new(0.0),
            peek_offset:      peek,
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
                stair_eye_offset: StairEyeOffset::new(0.0),
                peek_offset:      peek,
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
            "rotating facing to {dir:?} with a non-zero PeekOffset must NOT move the \
             facing-neutral peeked eye anchor (facing-invariant)"
        );
    }
}

#[test]
fn clamp_keeps_eye_in_cell() {
    use bevy::math::Vec2;

    let tuning = CombatTuning::default();
    let cell_x = 7_i32;
    let cell_y = 7_i32;
    let pos = position(cell_x, cell_y, 0);
    let st = stance(StanceKind::Standing);
    let f = facing(Direction::North);

    for (dx, dy) in [
        (5.0_f32, 5.0_f32),
        (-5.0, -5.0),
        (5.0, -5.0),
        (-5.0, 5.0),
        (0.49, 0.49),
        (-0.49, -0.49),
    ] {
        let peek = PeekOffset::new(Vec2::new(dx, dy));
        let eye = eye_anchor(
            &Observer {
                position:         &pos,
                stance:           &st,
                facing:           &f,
                stair_eye_offset: StairEyeOffset::new(0.0),
                peek_offset:      peek,
            },
            &tuning,
        );
        let (eye_cell, _) = pos_to_cell(eye);
        assert_eq!(
            (eye_cell.x, eye_cell.y),
            (cell_x, cell_y),
            "PeekOffset({dx}, {dy}) must keep the eye in the observer's cell \
             ({cell_x}, {cell_y}); got ({}, {}) instead — clamp failed",
            eye_cell.x,
            eye_cell.y
        );
    }
}
