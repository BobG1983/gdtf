use super::support::*;
use crate::march::MarchGrids;

#[test]
fn repeat_calls_are_identical() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
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

    let open = OccupancyGrid::new();
    let open_cover = CoverLedger::new();
    let clear_a = has_los(
        &observer,
        &target,
        MarchGrids {
            occupancy: &open,
            surface:   &surface,
            cover:     &open_cover,
        },
        &tuning,
        no_dead(),
    );
    let clear_b = has_los(
        &observer,
        &target,
        MarchGrids {
            occupancy: &open,
            surface:   &surface,
            cover:     &open_cover,
        },
        &tuning,
        no_dead(),
    );
    assert_eq!(
        clear_a, clear_b,
        "an open line of sight must be deterministic"
    );
    assert!(*clear_a, "the open geometry is CLEAR");

    let mut blocked_cover = CoverLedger::new();
    blocked_cover.insert(key(5, 5, 0), cover_entry(HeightBand::High));
    let blocked_a = has_los(
        &observer,
        &target,
        MarchGrids {
            occupancy: &open,
            surface:   &surface,
            cover:     &blocked_cover,
        },
        &tuning,
        no_dead(),
    );
    let blocked_b = has_los(
        &observer,
        &target,
        MarchGrids {
            occupancy: &open,
            surface:   &surface,
            cover:     &blocked_cover,
        },
        &tuning,
        no_dead(),
    );
    assert_eq!(
        blocked_a, blocked_b,
        "a blocked line of sight must be deterministic"
    );
    assert!(!*blocked_a, "the walled geometry is BLOCKED");
}
