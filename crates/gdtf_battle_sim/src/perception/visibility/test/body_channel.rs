use super::support::*;
use crate::occupancy::TerrainKind;

// The disc's storey range reads authored terrain and occupant slots, so a body-only storey
// needs one authored cell before the probe reaches it at all.
fn author_the_target_storey(grid: &mut OccupancyGrid) {
    grid.set_terrain(key(0, 0, 0), TerrainKind::Wall);
}

#[test]
fn a_body_is_seen_by_the_same_rule_as_a_living_prone_occupant() {
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
    let union_over = |occupancy: &OccupancyGrid| {
        union_fov(
            &make_observers(),
            occupancy,
            &surface,
            &cover,
            &tuning,
            no_dead(),
        )
        .contains(&target_cell)
    };

    let mut living_prone = OccupancyGrid::new();
    author_the_target_storey(&mut living_prone);
    place_occupant(
        &mut living_prone,
        target_cell,
        spawn_entity(),
        HeightBand::Low,
    );
    let living_seen = union_over(&living_prone);

    let mut body = OccupancyGrid::new();
    author_the_target_storey(&mut body);
    place_body(&mut body, target_cell, spawn_entity(), HeightBand::Low);
    let body_seen = union_over(&body);

    let mut no_band = OccupancyGrid::new();
    author_the_target_storey(&mut no_band);
    no_band.set_occupant(target_cell, Some(spawn_entity()));
    let no_band_seen = union_over(&no_band);

    assert_eq!(
        body_seen, living_seen,
        "the fog probe reads a body's floor band, so a body's cell is seen by the same rule as \
         a living prone occupant — body {body_seen}, living prone {living_seen}, no band \
         {no_band_seen}",
    );
    assert_ne!(
        body_seen, no_band_seen,
        "a body's cell is NOT probed at the standing stance fallback — body {body_seen}, \
         living prone {living_seen}, no band {no_band_seen}",
    );
}
