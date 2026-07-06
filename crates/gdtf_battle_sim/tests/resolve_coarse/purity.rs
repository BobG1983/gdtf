//! `resolve_coarse` purity: seed determinism, no combat-state mutation, and
//! reading the passed grid (AC #5 / AC #6 / AC #3).

use bevy::prelude::{Entity, World};
use gdtf_battle_sim::{
    cone::ConeAngle,
    cover::{CoverLedger, HeightBand},
    ganger::{Hp, Wounds},
    prelude::{OccupancyGrid, Tu},
    resolve_coarse::{ShotKind, ShotOutcome, resolve_coarse},
    surface::SurfaceGrid,
    test_support::{SituationBuilder, ganger_at, key, shot_rng},
    tuning::CombatTuning,
};

use super::harness::*;

// --- AC #5 — determinism (same seed → same outcome). ---

/// The SAME `BattleSeed` yields the SAME `ShotOutcome` for identical inputs — a
/// seeded-replay property. Uses a NON-zero cone so the cone sample (and the ganger
/// part roll) genuinely draw from the RNG, then asserts two fresh seeds reproduce.
#[test]
fn same_seed_yields_same_outcome() {
    let tuning = CombatTuning::default();
    let target = key(5, 2, 0);
    let mut occupancy = OccupancyGrid::new();
    // Hand-build the maintained occupant: a HIGH-banded ganger so any round impacts.
    let mut probe_world = World::new();
    let entity = probe_world.spawn_empty().id();
    occupancy.set_occupant(target, Some(entity));
    occupancy.set_occupant_band(target, Some(HeightBand::High));
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();

    // A non-zero cone so the sample genuinely draws (radius + azimuth) and the part
    // roll draws too — both must reproduce under the same seed.
    let shot = standing_shot(key(2, 2, 0), target, None, ConeAngle::new(0.05));
    let run = |seed: u64| -> ShotOutcome {
        let mut rng = shot_rng(seed);
        resolve_coarse(
            &shot,
            &occupancy,
            &surface,
            &cover,
            &tuning,
            &mut rng,
            no_dead(),
        )
    };

    let a = run(0xC0_FFEE);
    let b = run(0xC0_FFEE);
    assert_eq!(a, b, "the same seed reproduces the same ShotOutcome");
    // The replay struck the ganger and rolled a part (the RNG path actually ran).
    assert_eq!(a.kind, ShotKind::Ganger(entity));
    assert!(
        a.body_part.is_some(),
        "the ganger replay carries a body part"
    );
}

// --- AC #6 — no mutation; sim-unit positions; no damage / TU bookkeeping. ---

/// A snapshot of the combat state `resolve_coarse` must NOT touch — the target's
/// `Hp` / `Wounds` / worn-piece `ArmorIntegrity` (E3, on the related `Wears` piece
/// entities since GTW-323) and the shooter's `Tu` (E4).
struct CombatSnapshot {
    hp:     Hp,
    wounds: Wounds,
    /// The target's worn-piece integrities (sorted) — read off the `Wears` piece
    /// entities (GTW-323 / ADR-0004), not a removed `WornArmor` component.
    armor:  Vec<i32>,
    tu:     Tu,
}

/// Read the [`CombatSnapshot`] for `(target, shooter)` off the world, or `None` if
/// any component is missing (keeping the test panic-free).
fn combat_snapshot(world: &World, target: Entity, shooter: Entity) -> Option<CombatSnapshot> {
    use bevy::ecs::relationship::RelationshipTarget;

    let wears = world.get::<gdtf_battle_sim::armor::Wears>(target)?;
    let mut armor: Vec<i32> = wears
        .iter()
        .filter_map(|piece| {
            world
                .get::<gdtf_battle_sim::armor::ArmorIntegrity>(piece)
                .map(|c| **c)
        })
        .collect();
    armor.sort_unstable();
    Some(CombatSnapshot {
        hp: *world.get::<Hp>(target)?,
        wounds: *world.get::<Wounds>(target)?,
        armor,
        tu: *world.get::<Tu>(shooter)?,
    })
}

/// `resolve_coarse` mutates NOTHING: after the call the struck ganger's `Hp` /
/// `Wounds` / worn-piece integrities and the shooter's `Tu` are unchanged (E3 / E4
/// boundary).
#[test]
fn resolve_coarse_mutates_no_combat_state() {
    let shooter_at = key(2, 2, 0);
    let target_at = key(5, 2, 0);
    let situation = SituationBuilder::new()
        .with_gangers([ganger_at(shooter_at, 0), ganger_at(target_at, 1)])
        .build();
    let Some((mut app, setup)) = run_setup(situation) else {
        return;
    };
    let shooter_entity: Entity = setup.occupants[0].occupant;
    let target_entity: Entity = setup.occupants[1].occupant;

    let Some(before) = combat_snapshot(app.world(), target_entity, shooter_entity) else {
        return;
    };

    // Publish the maintained occupant band (HIGH so any round impacts).
    {
        let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() else {
            return;
        };
        grid.set_occupant_band(target_at, Some(HeightBand::High));
    }

    // Resolve a shot at the target; assert it struck (so the no-mutation claim is
    // about a real hit) and the trajectory is a sim-space unit vector (zero px).
    let tuning = CombatTuning::default();
    let mut rng = shot_rng(0xBEEF);
    {
        let world: &World = app.world();
        let (Some(occupancy), Some(surface), Some(cover)) = (
            world.get_resource::<OccupancyGrid>(),
            world.get_resource::<SurfaceGrid>(),
            world.get_resource::<CoverLedger>(),
        ) else {
            return;
        };
        let shot = standing_shot(shooter_at, target_at, None, zero_cone());
        let outcome = resolve_coarse(
            &shot,
            occupancy,
            surface,
            cover,
            &tuning,
            &mut rng,
            no_dead(),
        );
        assert_eq!(
            outcome.kind,
            ShotKind::Ganger(target_entity),
            "the shot struck the target (so the no-mutation claim is about a real hit)",
        );
        let traj = outcome.trajectory.vec();
        assert!(
            (traj.length() - 1.0).abs() < 1.0e-4,
            "the trajectory is a sim-space unit direction",
        );
    }

    // The combat state is unchanged — no damage (E3), no TU spend (E4).
    let Some(after) = combat_snapshot(app.world(), target_entity, shooter_entity) else {
        return;
    };
    assert_eq!(
        after.hp, before.hp,
        "the target's Hp is unchanged (no damage)"
    );
    assert_eq!(
        after.wounds, before.wounds,
        "the target's Wounds are unchanged (no severity)",
    );
    assert_eq!(
        after.armor, before.armor,
        "the target's worn-piece integrities are unchanged (no degradation)",
    );
    assert_eq!(
        after.tu, before.tu,
        "the shooter's Tu is unchanged (no fire economy)"
    );
}

// --- AC #3 — the passed grid is READ, not rebuilt. ---

/// `resolve_coarse` reads the passed grids — the SAME hand-built grid instance
/// drives the outcome (no per-shot rebuild path exists in the signature). Marking a
/// HIGH occupant in the grid we pass produces a Ganger outcome carrying THAT
/// entity; an empty grid over the same geometry produces a Miss — so the result
/// provably came from the grid we handed in, not a fresh internal one.
#[test]
fn resolve_coarse_reads_the_passed_grid() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let target = key(5, 2, 0);

    let mut probe_world = World::new();
    let entity = probe_world.spawn_empty().id();

    // Grid A: a HIGH occupant marked at the target (the maintained state).
    let mut grid_with_occupant = OccupancyGrid::new();
    grid_with_occupant.set_occupant(target, Some(entity));
    grid_with_occupant.set_occupant_band(target, Some(HeightBand::High));
    // Grid B: empty (no occupant).
    let grid_empty = OccupancyGrid::new();

    let shot = standing_shot(key(2, 2, 0), target, None, zero_cone());
    let run = |grid: &OccupancyGrid| -> ShotOutcome {
        let mut rng = shot_rng(1);
        resolve_coarse(&shot, grid, &surface, &cover, &tuning, &mut rng, no_dead())
    };

    let with_occupant = run(&grid_with_occupant);
    let empty = run(&grid_empty);

    assert_eq!(
        with_occupant.kind,
        ShotKind::Ganger(entity),
        "the passed grid's occupant drives the outcome — the grid was READ",
    );
    assert_eq!(
        empty.kind,
        ShotKind::Miss,
        "an empty passed grid yields a Miss — no internal rebuild conjured an occupant",
    );
}
