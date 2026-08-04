use super::support::*;
use crate::{ganger::Direction, los::probe::eye_anchor, march::MarchGrids};

#[test]
fn eye_anchor_is_facing_invariant() {
    let tuning = CombatTuning::default();
    let pos = position(5, 5, 0);
    let st = stance(StanceKind::Standing);

    let north_facing = facing(Direction::North);
    let reference = eye_anchor(
        &Observer {
            position:         &pos,
            stance:           &st,
            facing:           &north_facing,
            stair_eye_offset: StairEyeOffset::new(0.0),
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
                stair_eye_offset: StairEyeOffset::new(0.0),
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
            "rotating facing to {dir:?} must NOT move the facing-neutral eye anchor",
        );
    }
}

#[test]
fn verdict_is_facing_invariant() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let occupancy = OccupancyGrid::new();
    let mut cover = CoverLedger::new();
    cover.insert(key(5, 5, 0), cover_entry(HeightBand::High));

    let from_pos = position(2, 5, 0);
    let from_stance = stance(StanceKind::Standing);
    let to_pos = position(8, 5, 0);
    let to_stance = stance(StanceKind::Standing);
    let target = Target {
        position: &to_pos,
        stance:   &to_stance,
    };

    let mut verdicts = Vec::new();
    for dir in [
        Direction::East,
        Direction::West,
        Direction::North,
        Direction::South,
    ] {
        let f = facing(dir);
        let observer = Observer {
            position:         &from_pos,
            stance:           &from_stance,
            facing:           &f,
            stair_eye_offset: StairEyeOffset::new(0.0),
            peek_offset:      PeekOffset::default(),
        };
        verdicts.push(*has_los(
            &observer,
            &target,
            MarchGrids {
                occupancy: &occupancy,
                surface:   &surface,
                cover:     &cover,
            },
            &tuning,
            no_dead(),
        ));
    }

    assert!(
        verdicts.windows(2).all(|w| w[0] == w[1]),
        "the has_los verdict must be identical across all observer facings, got {verdicts:?}",
    );
    assert!(
        !verdicts[0],
        "the control geometry must be BLOCKED (a HIGH wall between)"
    );
}
