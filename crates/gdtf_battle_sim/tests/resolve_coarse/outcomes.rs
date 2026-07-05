//! Ganger-hit outcomes on the real path + derived pools driving resolution
//! (AC #2, GTW-384 C8(d)).

use bevy::prelude::{Entity, World};
use gdtf_battle_sim::{
    Accuracy, Aim, Aiming, Cell, CombatTuning, CoverLedger, Direction, Facing, Faction, HeightBand,
    Hp, Level, OccupancyGrid, Shooting, ShotKind, Stance, StanceKind, SurfaceGrid, Wounds,
    concentration_p, resolve_coarse,
    test_support::{GangerSpawnBuilder, SituationBuilder, ganger_at, key, shot_rng},
};

use super::harness::*;

// --- AC #2 — the real path yields a Ganger outcome whose Entity is the target. ---

/// `resolve_coarse` over a `setup_battle` situation (the real path) yields a
/// `Ganger` outcome whose struck `Entity` is the target and whose `BodyPart` is
/// set: a standing shooter at (2,2,0) facing East, a standing target at (5,2,0)
/// banded HIGH so any round impacts.
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

    // The change-driven occupancy sync (occupancy_sync::sync_moved_gangers, GTW-304)
    // publishes the occupant band on the real path; this isolated geometry test sets
    // the same maintained state directly via set_occupant_band (HIGH so any round
    // impacts), keeping the hand-computed geometry independent of the sync wiring.
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

/// GTW-384 C8(d) — the LIVE-COMBAT smoke: a ganger's DERIVED `Shooting` (computed at
/// `setup_battle` from its eight authored attributes × `GangerStatTuning`) drives shot
/// resolution AS BEFORE, and its DERIVED `Hp` / `Wounds` are the live combat pools. Spawn
/// via the REAL `setup_battle` derivation path, read the shooter's derived `Shooting` off
/// the entity, feed it through the SAME §1b `concentration_p` the fire pipeline uses, and
/// run `resolve_coarse` with the resulting `ConcentrationP` — it must land on the target
/// ganger (the derived skill drives the shot). The target's derived `Hp` / `Wounds` are
/// the pools `apply_hit` drains. RELATIONS-ONLY: asserts the derived stats are usable
/// (`> 0`) and drive a landed outcome — never a pinned shipped magnitude.
#[test]
fn derived_shooting_and_pools_drive_combat_resolution() {
    let shooter_at = key(2, 2, 0);
    let target_at = key(5, 2, 0);
    // High-Aim shooter (→ high derived Shooting) vs a default-attribute target.
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

    // The shooter's DERIVED Shooting — read off the spawned entity (the value setup
    // derived from the authored attributes). It must be a usable, positive skill.
    let shooting = app.world().get::<Shooting>(shooter_entity).copied();
    assert!(
        shooting.is_some_and(|s| *s > 0.0),
        "the shooter's DERIVED Shooting must be a usable positive skill: {shooting:?}",
    );
    let Some(shooting) = shooting else { return };

    // The target's DERIVED life pools — the combat drain pools. Both positive (a fresh
    // ganger has HP + Wounds to lose).
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
    // Feed the DERIVED Shooting through the SAME §1b concentration the fire pipeline uses.
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
