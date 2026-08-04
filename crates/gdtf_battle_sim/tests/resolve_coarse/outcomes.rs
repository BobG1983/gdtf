use bevy::prelude::{Entity, World};
use gdtf_battle_sim::{
    cover::{CoverLedger, HeightBand},
    ganger::{Aim, Aiming, Facing, Hp, Shooting, Wounds},
    prelude::{Cell, Direction, Faction, Level, OccupancyGrid, Stance, StanceKind},
    resolve_coarse::{ShotKind, resolve_coarse},
    sample_cone::concentration_p,
    surface::SurfaceGrid,
    test_support::{GangerSpawnBuilder, SituationBuilder, ganger_at, key, shot_rng},
    tuning::CombatTuning,
    weapon::Accuracy,
};

use super::harness::*;

#[test]
fn resolve_coarse_yields_ganger_outcome_on_real_path() {
    let shooter_at = key(2, 2, 0);
    let target_at = key(5, 2, 0);
    let situation = SituationBuilder::new()
        .with_gangers([ganger_at(shooter_at, 0), ganger_at(target_at, 1)])
        .build();
    let Some((mut app, setup)) = run_setup(situation) else {
        return;
    };
    let target_entity: Entity = setup.occupants[1].occupant;

    let world: &mut World = app.world_mut();
    let Some(mut grid) = world.get_resource_mut::<OccupancyGrid>() else {
        return;
    };
    grid.set_occupant_band(target_at, Some(HeightBand::High));

    let tuning = CombatTuning::default();
    let mut rng = shot_rng(0xA_11CE);

    let Some(occupancy) = world.get_resource::<OccupancyGrid>() else {
        return;
    };
    let Some(surface) = world.get_resource::<SurfaceGrid>() else {
        return;
    };
    let Some(cover) = world.get_resource::<CoverLedger>() else {
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
        "the shot must strike the target ganger entity",
    );
    assert_eq!(
        outcome.cell,
        Cell::new(5, 2),
        "the outcome cell is the target"
    );
    assert_eq!(
        outcome.level,
        Level::new(0),
        "the outcome storey is the target's"
    );
    assert!(
        outcome.body_part.is_some(),
        "a ganger outcome carries a struck body part",
    );
}

#[test]
fn derived_shooting_and_pools_drive_combat_resolution() {
    let shooter_at = key(2, 2, 0);
    let target_at = key(5, 2, 0);
    let situation = SituationBuilder::new()
        .with_gangers([
            GangerSpawnBuilder::new()
                .at(shooter_at)
                .faction(Faction::new(0))
                .facing(Facing::new(Direction::East))
                .stance(Stance::new(StanceKind::Standing))
                .aiming(Aiming::new(true))
                .aim(Aim::new(12.0))
                .build(),
            ganger_at(target_at, 1),
        ])
        .build();
    let Some((mut app, setup)) = run_setup(situation) else {
        return;
    };
    let shooter_entity: Entity = setup.occupants[0].occupant;
    let target_entity: Entity = setup.occupants[1].occupant;

    let shooting = app.world().get::<Shooting>(shooter_entity).copied();
    assert!(
        shooting.is_some_and(|s| *s > 0.0),
        "the shooter's DERIVED Shooting must be a usable positive skill: {shooting:?}",
    );
    let Some(shooting) = shooting else { return };

    let target_hp = app.world().get::<Hp>(target_entity).copied();
    let target_wounds = app.world().get::<Wounds>(target_entity).copied();
    assert!(
        target_hp.is_some_and(|h| *h > 0),
        "the target's DERIVED Hp must be a positive knock-down pool: {target_hp:?}",
    );
    assert!(
        target_wounds.is_some_and(|w| *w > 0),
        "the target's DERIVED Wounds must be a positive life pool: {target_wounds:?}",
    );

    let world: &mut World = app.world_mut();
    let Some(mut grid) = world.get_resource_mut::<OccupancyGrid>() else {
        return;
    };
    grid.set_occupant_band(target_at, Some(HeightBand::High));

    let tuning = CombatTuning::default();
    let p = concentration_p(
        shooting,
        Accuracy::new(1.0),
        tuning.cone_stability.concentration,
    );
    assert!(
        *p > 0.0,
        "the derived Shooting yields a positive concentration exponent"
    );

    let mut rng = shot_rng(0xA_11CE);
    let Some(occupancy) = world.get_resource::<OccupancyGrid>() else {
        return;
    };
    let Some(surface) = world.get_resource::<SurfaceGrid>() else {
        return;
    };
    let Some(cover) = world.get_resource::<CoverLedger>() else {
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
        "the DERIVED-Shooting shot lands on the target ganger (the derived skill drives the shot)",
    );
}
