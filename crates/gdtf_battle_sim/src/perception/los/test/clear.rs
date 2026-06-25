//! AC: a clear line of sight, and an intact wall / slab / standing cover band that
//! sits strictly between blocks it.

use super::support::*;

/// An unobstructed eye→target with open cells between is CLEAR: the march flies past
/// the aim cell to a clean Miss, so nothing blocked before the target.
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
        &occupancy,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );
    assert!(*sighted, "an open eye→target line must be CLEAR");
}

/// An intact HIGH wall (cover band HIGH) sitting strictly between a standing eye and a
/// standing target BLOCKS sight: a HIGH round is not strictly higher than a HIGH wall,
/// so the march stops on the wall before reaching the target.
#[test]
fn high_wall_between_blocks() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let occupancy = OccupancyGrid::new();
    let mut cover = CoverLedger::new();
    // A HIGH wall midway between the standing eye and the standing target.
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
        &occupancy,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );
    assert!(!*sighted, "a HIGH wall strictly between must BLOCK sight");
}

/// An intact `Present` slab on the z-boundary strictly between an eye and a target on a
/// higher storey BLOCKS sight: the climbing eye→target ray is stopped at the slab
/// before it reaches the target cell.
#[test]
fn slab_between_levels_blocks() {
    let tuning = CombatTuning::default();
    let cover = CoverLedger::new();
    let occupancy = OccupancyGrid::new();
    let mut surface = SurfaceGrid::new();

    // Observer on L0, target on L1: the eye→aim ray climbs across the z=1 boundary.
    let from_pos = position(2, 5, 0);
    let from_stance = stance(StanceKind::Standing);
    let from_facing = facing(Direction::East);
    let to_pos = position(8, 5, 1);
    let to_stance = stance(StanceKind::Standing);

    // With NO slab the climb is clear; an intact slab on the crossed boundary blocks it.
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
        &occupancy,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );
    assert!(
        *open,
        "with no slab the climbing line of sight is CLEAR (control)"
    );

    // The march crosses into L1 around the middle of the span; seed the slab at every
    // L1 floor cell between the two so an intact slab is guaranteed on the crossed
    // boundary regardless of the exact crossing column (a band-free placement — no
    // magnitude pin).
    for x in 3..=7 {
        surface.set_slab(key(x, 5, 1), SlabState::Present);
    }
    let blocked = has_los(
        &observer,
        &target,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );
    assert!(
        !*blocked,
        "an intact slab on the crossed z-boundary must BLOCK sight"
    );
}
