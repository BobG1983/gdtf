//! AC: shot-pipeline parity — for a no-cover target whose `occupant_band` is
//! non-Standing, the probe's target aim z EQUALS
//! `target_aim_point(target_position, target_stance, occupancy.occupant_band(cell), tuning).z`,
//! i.e. the band-midpoint branch the live shot pipeline uses — NOT the bare-stance
//! silhouette-top branch (GTW-337 clause 4).

use super::support::*;
use crate::{central_axis::target_aim_point, los::probe::aim_anchor};

/// A no-cover Crouching target with a published MID `occupant_band`: the probe's aim
/// anchor z EQUALS the shot pipeline's `target_aim_point(.., occupant_band, ..).z`
/// (the band-midpoint branch), and is DISTINCT from the bare-stance silhouette-top
/// branch (`target_aim_point(.., None, ..)`) — proving the probe routes the band, not
/// a bare cover peek.
#[test]
fn aim_z_matches_occupant_band_not_bare_stance() {
    let tuning = CombatTuning::default();
    let cover = CoverLedger::new(); // NO cover at the target cell.

    let target_cell = key(8, 5, 0);
    let occupant = spawn_entity();
    let mut occupancy = OccupancyGrid::new();
    // A non-Standing occupant band (Crouching → MID), no cover.
    place_occupant(&mut occupancy, target_cell, occupant, HeightBand::Mid);

    let to_pos = position(8, 5, 0);
    let to_stance = stance(StanceKind::Crouching);
    let target = Target {
        position: &to_pos,
        stance:   &to_stance,
    };

    // The shot pipeline's aim point for this target, using the published occupant band
    // (the exact derivation TargetGeometry::compose feeds target_aim_point).
    let pipeline_band = occupancy.occupant_band(&target_cell);
    let pipeline_aim = target_aim_point(to_pos, to_stance, pipeline_band, &tuning);

    // The bare-stance silhouette-top branch (what a bare cover.peek that found nothing
    // would degrade to) — a DIFFERENT z.
    let bare_stance_aim = target_aim_point(to_pos, to_stance, None, &tuning);

    let probe_aim = aim_anchor(&target, &occupancy, &cover, &tuning);

    assert_eq!(
        probe_aim.z.to_bits(),
        pipeline_aim.z.to_bits(),
        "the probe's aim z must EQUAL the shot pipeline's occupant_band-derived aim z",
    );
    assert_ne!(
        pipeline_aim.z.to_bits(),
        bare_stance_aim.z.to_bits(),
        "the occupant_band aim z must DIFFER from the bare-stance silhouette-top z \
         (otherwise the parity test proves nothing)",
    );
}
