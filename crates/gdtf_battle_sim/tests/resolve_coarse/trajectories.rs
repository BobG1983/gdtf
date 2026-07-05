//! Non-ganger shot outcomes: miss trajectory, cover entry, ground exit, and
//! `ShotOutcome` variant construction (AC #4 / AC #1).

use bevy::prelude::World;
use gdtf_battle_sim::{
    ArmorHardness, ArmorProtection, BodyPart, Cell, CombatTuning, ConeAngle, CoverEntry, CoverHp,
    CoverLedger, Direction, Facing, HeightBand, Level, OccupancyGrid, Position, PriorShots,
    RecoilClimb, RecoilGrowth, ShotInputs, ShotKind, ShotOutcome, Stance, StanceKind, SurfaceGrid,
    resolve_coarse,
    test_support::{key, shot_rng},
};

use super::harness::*;

// --- AC #4 — Miss / Cover / Ground over composed geometry. ---

/// A shot that clears everything returns a `Miss` (no struck object), and the
/// trajectory is still carried for presenter FX. Empty grids, flat East: nothing in
/// the path, the round leaves laterally.
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
    // The trajectory is a real unit vector (carried for FX even on a miss).
    let traj = outcome.trajectory.vec();
    assert!(
        (traj.length() - 1.0).abs() < 1.0e-4,
        "the trajectory is a unit direction, carried even on a miss",
    );
}

/// A shot into cover returns a `Cover` outcome carrying the struck `CoverEntry`. A
/// MID cover stands in the path; the round (dead-center on the aim axis) impacts it.
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

    // Aim at the cover cell's own band midpoint (a deliberately-shot crate).
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

/// A downward shot off the grid bottom returns a `Ground` outcome carrying the
/// surface cell it exited through. Straight down (-z) from (4,4,0).
#[test]
fn downward_shot_off_bottom_returns_ground() {
    let tuning = CombatTuning::default();
    let occupancy = OccupancyGrid::new();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let mut rng = shot_rng(13);
    let (prior_shots, recoil_climb, recoil_growth) = no_recoil();

    // Target directly below the shooter so the muzzle→aim axis points down (-z).
    // North-facing (not the East helper) to keep this geometry identical to GTW-172.
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

// --- AC #1 — each ShotOutcome variant constructs and inspects. ---

/// Each `ShotKind` variant constructs and its struck payload reads back, and a
/// `ShotOutcome` carries every named field — the type-surface check (AC #1).
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

    // Ganger variant — carries the Entity; the body part is Some only here.
    let ganger = ShotOutcome {
        kind:       ShotKind::Ganger(entity),
        cell:       Cell::new(3, 4),
        level:      Level::new(2),
        body_part:  Some(BodyPart::Torso),
        band:       HeightBand::Mid,
        muzzle:     gdtf_battle_sim::SimPos::new(2.5, 2.5, 0.5),
        trajectory: trajectory_unit_x(),
    };
    assert_eq!(ganger.kind, ShotKind::Ganger(entity));
    assert_eq!(ganger.body_part, Some(BodyPart::Torso));
    assert_eq!(ganger.cell, Cell::new(3, 4));
    assert_eq!(ganger.level, Level::new(2));

    // Cover / Slab / Ground / Miss variants carry their struck object (or none).
    assert!(matches!(ShotKind::Cover(entry), ShotKind::Cover(_)));
    assert_eq!(ShotKind::Slab(surface_cell), ShotKind::Slab(surface_cell));
    assert_eq!(
        ShotKind::Ground(surface_cell),
        ShotKind::Ground(surface_cell)
    );
    assert_eq!(ShotKind::Miss, ShotKind::Miss);
}

/// A unit-X `ShotDir` for the type-surface test — built through the public cone
/// sampler under a zero cone (dead-center on a +X aim axis), so it exercises the
/// real `ShotDir` constructor rather than a hand-built private value.
fn trajectory_unit_x() -> gdtf_battle_sim::ShotDir {
    use gdtf_battle_sim::{SimPos, climb_aim_dir, sample_cone_vector};
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
