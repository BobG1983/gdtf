use super::support::*;
use crate::march::MarchGrids;

#[test]
fn a_body_lying_on_a_cell_answers_the_aim_band_after_the_occupant() {
    let tuning = CombatTuning::default();
    let cover = CoverLedger::new();
    let (body, occupant) = spawn_two_entities();

    let at = key(8, 5, 0);
    let to_pos = position(8, 5, 0);
    let to_stance = stance(StanceKind::Standing);
    let target = Target {
        position: &to_pos,
        stance:   &to_stance,
    };

    let mut body_only = OccupancyGrid::new();
    body_only.set_body(at, Some(BodyOcclusion::new(body, HeightBand::Mid)));
    let body_aim = super::super::probe::aim_anchor(&target, &body_only, &cover, &tuning);
    assert_eq!(
        body_aim,
        target_aim_point(to_pos, to_stance, Some(HeightBand::Mid), &tuning),
        "a body's cell is aimed at the band the body channel recorded — found {body_aim:?}",
    );

    let mut occupied = body_only.clone();
    place_occupant(&mut occupied, at, occupant, HeightBand::High);
    let occupied_aim = super::super::probe::aim_anchor(&target, &occupied, &cover, &tuning);
    assert_eq!(
        occupied_aim,
        target_aim_point(to_pos, to_stance, Some(HeightBand::High), &tuning),
        "a living ganger standing on a body's cell is aimed at its own silhouette, not at the \
         body — found {occupied_aim:?}",
    );

    let empty = OccupancyGrid::new();
    let empty_aim = super::super::probe::aim_anchor(&target, &empty, &cover, &tuning);
    assert_eq!(
        empty_aim,
        target_aim_point(to_pos, to_stance, None, &tuning),
        "a cell with nothing on it keeps the stance fallback — found {empty_aim:?}",
    );
}

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
        MarchGrids {
            occupancy: &occupancy,
            surface:   &surface,
            cover:     &cover,
        },
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
        MarchGrids {
            occupancy: &occupancy,
            surface:   &surface,
            cover:     &cover,
        },
        &tuning,
        corpse_predicate,
    );
    assert!(
        *through,
        "a corpse strictly between must NOT block sight (passthrough)"
    );
}
