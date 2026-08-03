use super::support::*;
use crate::central_axis::target_aim_point;

#[test]
fn band_resolved_aim_z_is_not_cell_center() {
    let tuning = CombatTuning::default();

    let target_cell = key(8, 5, 0);
    let mut occupancy = OccupancyGrid::new();
    place_occupant(&mut occupancy, target_cell, spawn_entity(), HeightBand::Mid);

    let banded_aim = target_aim_point(
        Position::new(target_cell),
        stance(StanceKind::Standing),
        occupancy.occupant_band(&target_cell),
        &tuning,
    );

    let cell_center_z = crate::metric::cell_center(Cell::new(8, 5), Level::new(0)).z;

    assert_ne!(
        banded_aim.z.to_bits(),
        cell_center_z.to_bits(),
        "the occupant-band aim z must DIFFER from a generic cell-center z (z = level)"
    );
}

#[test]
fn union_verdict_depends_on_the_published_band() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let mut cover = CoverLedger::new();

    let target_cell = key(8, 5, 0);
    let edge_cell = key(7, 5, 1);
    cover.insert(edge_cell, cover_entry(HeightBand::High));

    let (pos, st, fc) = alive_observer_at(2, 5, 1);
    let make_observers = || {
        [FovObserver {
            position:         &pos,
            stance:           &st,
            facing:           &fc,
            life:             LifeState::Alive,
            stair_eye_offset: StairEyeOffset::new(0.0),
        }]
    };

    let mut occupancy_banded = OccupancyGrid::new();
    place_occupant(
        &mut occupancy_banded,
        target_cell,
        spawn_entity(),
        HeightBand::Mid,
    );
    let visible_banded = union_fov(
        &make_observers(),
        &occupancy_banded,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );

    let mut occupancy_no_band = OccupancyGrid::new();
    occupancy_no_band.set_occupant(target_cell, Some(spawn_entity()));
    let visible_no_band = union_fov(
        &make_observers(),
        &occupancy_no_band,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );

    let banded_aim_z = target_aim_point(
        Position::new(target_cell),
        stance(StanceKind::Standing),
        Some(HeightBand::Mid),
        &tuning,
    )
    .z;
    let bare_aim_z = target_aim_point(
        Position::new(target_cell),
        stance(StanceKind::Standing),
        None,
        &tuning,
    )
    .z;
    assert_ne!(
        banded_aim_z.to_bits(),
        bare_aim_z.to_bits(),
        "the MID-band aim z and the bare-stance aim z are distinct rays"
    );
    assert_ne!(
        visible_banded.contains(&target_cell),
        visible_no_band.contains(&target_cell),
        "the union's verdict for the crouched occupant FLIPS on whether its band is \
         published — proving union_fov resolves the occupant band, not a generic anchor"
    );
}
