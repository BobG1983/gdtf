use bevy::prelude::World;
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection, BodyPart},
    cone::{ConeAngle, PriorShots},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    ganger::Facing,
    prelude::{Cell, Direction, Level, OccupancyGrid, Position, Stance, StanceKind},
    resolve_coarse::{ShotInputs, ShotKind, ShotOutcome, resolve_coarse},
    stability::RecoilGrowth,
    surface::SurfaceGrid,
    test_support::{key, shot_rng},
    tuning::{CombatTuning, RecoilClimb},
};

use super::harness::*;

#[test]
fn clearing_shot_returns_miss_carrying_trajectory() {
    let tuning = CombatTuning::default();
    let occupancy = OccupancyGrid::new();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let mut rng = shot_rng(7);

    let shot = standing_shot(key(2, 2, 0), key(5, 2, 0), None, zero_cone());
    let outcome = resolve_coarse(
        &shot,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        &mut rng,
        no_dead(),
    );

    assert_eq!(
        outcome.kind,
        ShotKind::Miss,
        "an empty path clears to a Miss"
    );
    assert!(
        outcome.body_part.is_none(),
        "a non-ganger outcome carries no body part",
    );
    let traj = outcome.trajectory.vec();
    assert!(
        (traj.length() - 1.0).abs() < 1.0e-4,
        "the trajectory is a unit direction, carried even on a miss",
    );
}

#[test]
fn shot_into_cover_returns_cover_entry() {
    let tuning = CombatTuning::default();
    let occupancy = OccupancyGrid::new();
    let surface = SurfaceGrid::new();

    let cover_cell = key(5, 2, 0);
    let entry = CoverEntry::seeded(
        CoverHp::new(80),
        HeightBand::High,
        ArmorProtection::new(6),
        ArmorHardness::new(3),
    );
    let mut cover = CoverLedger::new();
    cover.insert(cover_cell, entry);

    let mut rng = shot_rng(99);

    let shot = standing_shot(
        key(2, 2, 0),
        cover_cell,
        Some(HeightBand::High),
        zero_cone(),
    );
    let outcome = resolve_coarse(
        &shot,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        &mut rng,
        no_dead(),
    );

    assert_eq!(
        outcome.kind,
        ShotKind::Cover(entry),
        "the shot must strike the cover, carrying its CoverEntry",
    );
    assert!(
        outcome.body_part.is_none(),
        "a cover outcome carries no body part",
    );
}

#[test]
fn downward_shot_off_bottom_returns_ground() {
    let tuning = CombatTuning::default();
    let occupancy = OccupancyGrid::new();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let mut rng = shot_rng(13);
    let (prior_shots, recoil_climb, recoil_growth) = no_recoil();

    let shot = ShotInputs {
        shooter_position: Position::new(key(4, 4, 1)),
        shooter_facing: Facing::new(Direction::North),
        shooter_stance: Stance::new(StanceKind::Standing),
        target_position: Position::new(key(4, 4, 0)),
        target_stance: Stance::new(StanceKind::Standing),
        cover_band: None,
        cone: zero_cone(),
        p: some_p(),
        prior_shots,
        recoil_climb,
        recoil_growth,
    };
    let outcome = resolve_coarse(
        &shot,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        &mut rng,
        no_dead(),
    );

    assert!(
        matches!(outcome.kind, ShotKind::Ground(_)),
        "a shot diving off the grid bottom strikes the Ground, got {:?}",
        outcome.kind,
    );
    assert!(
        outcome.body_part.is_none(),
        "a ground outcome carries no body part",
    );
}

#[test]
fn shot_outcome_variants_construct_and_inspect() {
    let mut probe_world = World::new();
    let entity = probe_world.spawn_empty().id();
    let entry = CoverEntry::seeded(
        CoverHp::new(10),
        HeightBand::Mid,
        ArmorProtection::new(1),
        ArmorHardness::new(1),
    );
    let surface_cell = key(3, 4, 2);

    let ganger = ShotOutcome {
        kind:       ShotKind::Ganger(entity),
        cell:       Cell::new(3, 4),
        level:      Level::new(2),
        body_part:  Some(BodyPart::Torso),
        band:       HeightBand::Mid,
        muzzle:     gdtf_battle_sim::metric::SimPos::new(2.5, 2.5, 0.5),
        trajectory: trajectory_unit_x(),
    };
    assert_eq!(ganger.kind, ShotKind::Ganger(entity));
    assert_eq!(ganger.body_part, Some(BodyPart::Torso));
    assert_eq!(ganger.cell, Cell::new(3, 4));
    assert_eq!(ganger.level, Level::new(2));

    assert!(matches!(ShotKind::Cover(entry), ShotKind::Cover(_)));
    assert_eq!(ShotKind::Slab(surface_cell), ShotKind::Slab(surface_cell));
    assert_eq!(
        ShotKind::Ground(surface_cell),
        ShotKind::Ground(surface_cell)
    );
    assert_eq!(ShotKind::Miss, ShotKind::Miss);
}

fn trajectory_unit_x() -> gdtf_battle_sim::sample_cone::ShotDir {
    use gdtf_battle_sim::{
        central_axis::climb_aim_dir, prelude::SimPos, sample_cone::sample_cone_vector,
    };
    let mut rng = shot_rng(0);
    let axis = climb_aim_dir(
        SimPos::new(0.0, 0.0, 0.0),
        SimPos::new(1.0, 0.0, 0.0),
        PriorShots::first(),
        RecoilClimb::new(0.0),
        RecoilGrowth::new(0.0),
    );
    sample_cone_vector(axis, ConeAngle::new(0.0), some_p(), rng.rng())
}
