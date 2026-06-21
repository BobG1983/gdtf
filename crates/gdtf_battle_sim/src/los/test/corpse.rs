//! AC: a corpse on the sight line does NOT block; a living (or Downed) ganger DOES —
//! the dead-occupant predicate is reused verbatim from the wrapped march (GTW-317).

use super::support::*;

/// A ganger occupant strictly between the eye and the target, banded to the sight
/// line's band, BLOCKS sight while living — but with the same occupant marked a corpse
/// (the `is_dead` predicate returns `true` for it) sight passes THROUGH it and the
/// living target is CLEAR. Only the predicate changes between the two probes.
#[test]
fn corpse_passes_through_living_blocks() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();

    let (mid_occupant, target_occupant) = spawn_two_entities();

    // A HIGH-banded occupant midway on a standing↔standing (HIGH) sight line, plus the
    // banded target occupant so the standing target's aim resolves via its
    // occupant_band (the shot-pipeline band derivation, not the bare-stance branch).
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
        position: &from_pos,
        stance:   &from_stance,
        facing:   &from_facing,
    };
    let target = Target {
        position: &to_pos,
        stance:   &to_stance,
    };

    // Living mid occupant (no corpses) → it blocks the line of sight.
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

    // The SAME mid occupant marked a corpse → sight passes through it; the target is
    // alive, so the line is CLEAR. Only the mid occupant is dead, never the target.
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
