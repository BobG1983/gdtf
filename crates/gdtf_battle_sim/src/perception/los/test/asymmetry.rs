//! AC: asymmetric sight — a low-watcher/tall-target geometry that one direction sees
//! and the other does not, in the SAME world. The asymmetry falls out of anchoring
//! eye-vs-aim (the eye is the per-stance/level muzzle height; the aim is the target's
//! band-midpoint), with no extra rule — folded in HERE (GTW-337).

use super::support::*;

/// A low watcher (prone, on the ground storey) can SEE a tall target (one storey up),
/// but the tall target — looking back along the SAME line in the SAME world — cannot
/// see the low one, because an intact HIGH wall on the ground storey sits in the
/// descending-from-above line but the climbing-from-below line has already risen out
/// of that storey before it reaches the wall's cell.
#[test]
fn low_sees_tall_but_tall_blocked_by_ground_wall() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let occupancy = OccupancyGrid::new();
    // A HIGH wall on the GROUND storey (level 0), one cell short of the elevated entity.
    let mut cover = CoverLedger::new();
    cover.insert(key(7, 5, 0), cover_entry(HeightBand::High));

    // The LOW entity: prone on the ground storey. The TALL entity: prone one storey up.
    let low_pos = position(2, 5, 0);
    let low_stance = stance(StanceKind::Prone);
    let low_facing = facing(Direction::East);
    let tall_pos = position(8, 5, 1);
    let tall_stance = stance(StanceKind::Prone);
    let tall_facing = facing(Direction::West);

    let low_observer = Observer {
        position:         &low_pos,
        stance:           &low_stance,
        facing:           &low_facing,
        stair_eye_offset: StairEyeOffset::new(0.0),
    };
    let low_target = Target {
        position: &low_pos,
        stance:   &low_stance,
    };
    let tall_observer = Observer {
        position:         &tall_pos,
        stance:           &tall_stance,
        facing:           &tall_facing,
        stair_eye_offset: StairEyeOffset::new(0.0),
    };
    let tall_target = Target {
        position: &tall_pos,
        stance:   &tall_stance,
    };

    let low_to_tall = has_los(
        &low_observer,
        &tall_target,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );
    let tall_to_low = has_los(
        &tall_observer,
        &low_target,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );

    assert!(
        *low_to_tall,
        "the low watcher must SEE the tall target (CLEAR)"
    );
    assert!(
        !*tall_to_low,
        "the tall watcher must be BLOCKED looking back (asymmetry)"
    );
    assert_ne!(
        *low_to_tall, *tall_to_low,
        "the verdict must be asymmetric in the same world"
    );
}
