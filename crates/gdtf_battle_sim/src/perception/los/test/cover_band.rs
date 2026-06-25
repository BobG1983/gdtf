//! AC: a standing cover band that blocks the line vs one the line sails over —
//! decided by the round's band vs the cover's band, the §2 clearance semantics
//! reused through the wrapped march.

use super::support::*;

/// In the SAME world (a MID cover strictly between), a HIGH sight line sails over the
/// cover (CLEAR) while a LOW sight line impacts it (BLOCKED) — the blocks-vs-sails
/// pair, switched only by the endpoints' stance bands.
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

    // Standing eye → standing target: a HIGH line is strictly higher than the MID
    // cover, so it sails over → CLEAR.
    let stand = stance(StanceKind::Standing);
    let high_observer = Observer {
        position: &from_pos,
        stance:   &stand,
        facing:   &look,
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

    // Prone eye → prone target: a LOW line is NOT strictly higher than the MID cover,
    // so it impacts it before the target → BLOCKED.
    let prone = stance(StanceKind::Prone);
    let low_observer = Observer {
        position: &from_pos,
        stance:   &prone,
        facing:   &look,
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
