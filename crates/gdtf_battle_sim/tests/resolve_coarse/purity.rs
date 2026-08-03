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


#[test]
fn same_seed_yields_same_outcome() {
    let tuning = CombatTuning::default();
    let target = key(5, 2, 0);
    let mut occupancy = OccupancyGrid::new();
    let mut probe_world = World::new();
    let entity = probe_world.spawn_empty().id();
    occupancy.set_occupant(target, Some(entity));
    occupancy.set_occupant_band(target, Some(HeightBand::High));
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();

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
    assert_eq!(a.kind, ShotKind::Ganger(entity));
    assert!(
        a.body_part.is_some(),
        "the ganger replay carries a body part"
    );
}


struct CombatSnapshot {
    hp:     Hp,
    wounds: Wounds,
            armor:  Vec<i32>,
    tu:     Tu,
}

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

    {
        let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() else {
            return;
        };
        grid.set_occupant_band(target_at, Some(HeightBand::High));
    }

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


#[test]
fn resolve_coarse_reads_the_passed_grid() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let target = key(5, 2, 0);

    let mut probe_world = World::new();
    let entity = probe_world.spawn_empty().id();

    let mut grid_with_occupant = OccupancyGrid::new();
    grid_with_occupant.set_occupant(target, Some(entity));
    grid_with_occupant.set_occupant_band(target, Some(HeightBand::High));
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
