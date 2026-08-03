use super::support::*;

#[test]
fn corpse_passes_through_living_blocks() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();

    let (mid_occupant, target_occupant) = spawn_two_entities();

    let mut occupancy = OccupancyGrid::new();
    place_occupant(&mut occupancy, key(5, 5, 0), mid_occupant, HeightBand::High);
    place_occupant(
        &mut occupancy,
        key(8, 5, 0),
        target_occupant,
        HeightBand::High,
    );

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

    let living = has_los(
        &observer,
        &target,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );
    assert!(
        !*living,
        "a living ganger strictly between must BLOCK sight"
    );

    let corpse_predicate = move |e: bevy::prelude::Entity| e == mid_occupant;
    let through = has_los(
        &observer,
        &target,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        corpse_predicate,
    );
    assert!(
        *through,
        "a corpse strictly between must NOT block sight (passthrough)"
    );
}
