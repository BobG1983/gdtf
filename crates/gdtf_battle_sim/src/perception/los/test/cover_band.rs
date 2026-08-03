use super::support::*;

#[test]
fn mid_cover_sails_for_high_line_blocks_low_line() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let occupancy = OccupancyGrid::new();
    let mut cover = CoverLedger::new();
    cover.insert(key(5, 5, 0), cover_entry(HeightBand::Mid));

    let from_pos = position(2, 5, 0);
    let to_pos = position(8, 5, 0);
    let look = facing(Direction::East);

    let stand = stance(StanceKind::Standing);
    let high_observer = Observer {
        position:         &from_pos,
        stance:           &stand,
        facing:           &look,
        stair_eye_offset: StairEyeOffset::new(0.0),
        peek_offset:      PeekOffset::default(),
    };
    let high_target = Target {
        position: &to_pos,
        stance:   &stand,
    };
    let sails = has_los(
        &high_observer,
        &high_target,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );
    assert!(
        *sails,
        "a HIGH sight line must SAIL OVER a MID cover → CLEAR"
    );

    let prone = stance(StanceKind::Prone);
    let low_observer = Observer {
        position:         &from_pos,
        stance:           &prone,
        facing:           &look,
        stair_eye_offset: StairEyeOffset::new(0.0),
        peek_offset:      PeekOffset::default(),
    };
    let low_target = Target {
        position: &to_pos,
        stance:   &prone,
    };
    let blocked = has_los(
        &low_observer,
        &low_target,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );
    assert!(
        !*blocked,
        "a LOW sight line must IMPACT a MID cover → BLOCKED"
    );
}
