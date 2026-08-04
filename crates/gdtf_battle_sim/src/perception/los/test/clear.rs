use super::support::*;
use crate::march::MarchGrids;

#[test]
fn open_line_is_clear() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let occupancy = OccupancyGrid::new();

    let from_pos = position(2, 5, 0);
    let from_stance = stance(StanceKind::Standing);
    let from_facing = facing(Direction::East);
    let to_pos = position(8, 5, 0);
    let to_stance = stance(StanceKind::Standing);

    let observer = Observer {
        position:         &from_pos,
        stance:           &from_stance,
        facing:           &from_facing,
        stair_eye_offset: StairEyeOffset::new(0.0),
        peek_offset:      PeekOffset::default(),
    };
    let target = Target {
        position: &to_pos,
        stance:   &to_stance,
    };

    let sighted = has_los(
        &observer,
        &target,
        MarchGrids {
            occupancy: &occupancy,
            surface:   &surface,
            cover:     &cover,
        },
        &tuning,
        no_dead(),
    );
    assert!(*sighted, "an open eye→target line must be CLEAR");
}

#[test]
fn high_wall_between_blocks() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let occupancy = OccupancyGrid::new();
    let mut cover = CoverLedger::new();
    cover.insert(key(5, 5, 0), cover_entry(HeightBand::High));

    let from_pos = position(2, 5, 0);
    let from_stance = stance(StanceKind::Standing);
    let from_facing = facing(Direction::East);
    let to_pos = position(8, 5, 0);
    let to_stance = stance(StanceKind::Standing);

    let observer = Observer {
        position:         &from_pos,
        stance:           &from_stance,
        facing:           &from_facing,
        stair_eye_offset: StairEyeOffset::new(0.0),
        peek_offset:      PeekOffset::default(),
    };
    let target = Target {
        position: &to_pos,
        stance:   &to_stance,
    };

    let sighted = has_los(
        &observer,
        &target,
        MarchGrids {
            occupancy: &occupancy,
            surface:   &surface,
            cover:     &cover,
        },
        &tuning,
        no_dead(),
    );
    assert!(!*sighted, "a HIGH wall strictly between must BLOCK sight");
}

#[test]
fn slab_between_levels_blocks() {
    let tuning = CombatTuning::default();
    let cover = CoverLedger::new();
    let occupancy = OccupancyGrid::new();
    let mut surface = SurfaceGrid::new();

    let from_pos = position(2, 5, 0);
    let from_stance = stance(StanceKind::Standing);
    let from_facing = facing(Direction::East);
    let to_pos = position(8, 5, 1);
    let to_stance = stance(StanceKind::Standing);

    let observer = Observer {
        position:         &from_pos,
        stance:           &from_stance,
        facing:           &from_facing,
        stair_eye_offset: StairEyeOffset::new(0.0),
        peek_offset:      PeekOffset::default(),
    };
    let target = Target {
        position: &to_pos,
        stance:   &to_stance,
    };
    let open = has_los(
        &observer,
        &target,
        MarchGrids {
            occupancy: &occupancy,
            surface:   &surface,
            cover:     &cover,
        },
        &tuning,
        no_dead(),
    );
    assert!(
        *open,
        "with no slab the climbing line of sight is CLEAR (control)"
    );

    for x in 3..=7 {
        surface.set_slab(key(x, 5, 1), SlabState::Present);
    }
    let blocked = has_los(
        &observer,
        &target,
        MarchGrids {
            occupancy: &occupancy,
            surface:   &surface,
            cover:     &cover,
        },
        &tuning,
        no_dead(),
    );
    assert!(
        !*blocked,
        "an intact slab on the crossed z-boundary must BLOCK sight"
    );
}
