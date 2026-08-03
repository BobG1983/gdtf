use super::support::*;
use crate::{central_axis::target_aim_point, los::probe::aim_anchor};

#[test]
fn aim_z_matches_occupant_band_not_bare_stance() {
    let tuning = CombatTuning::default();
    let cover = CoverLedger::new(); 

    let target_cell = key(8, 5, 0);
    let occupant = spawn_entity();
    let mut occupancy = OccupancyGrid::new();
    place_occupant(&mut occupancy, target_cell, occupant, HeightBand::Mid);

    let to_pos = position(8, 5, 0);
    let to_stance = stance(StanceKind::Crouching);
    let target = Target {
        position: &to_pos,
        stance:   &to_stance,
    };

    let pipeline_band = occupancy.occupant_band(&target_cell);
    let pipeline_aim = target_aim_point(to_pos, to_stance, pipeline_band, &tuning);

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
