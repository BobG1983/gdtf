//! AC2 — fail-closed cases: an empty magazine or a downed shooter fires nothing and
//! mutates nothing.

use super::support::*;

/// AC2 — fail-closed when the magazine is empty: an empty volley, NO TU charge,
/// NO draw, NO mutation.
#[test]
fn empty_magazine_fires_nothing_and_mutates_nothing() {
    let mut world = World::new();
    let tuning = CombatTuning::default();
    let mode = single_mode(0.2, 3);
    let shooter = spawn_shooter(
        &mut world,
        ShooterSpec {
            x: 5,
            y: 5,
            tu: 200,
            tu_max: 100,
            ammo: 0,
            mode,
            aiming: false,
        },
    );

    let occupancy = OccupancyGrid::new();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let mut r = rng();
    let mut fresh = rng();

    let mut state: SystemState<(ShooterQuery, TargetQuery)> = SystemState::new(&mut world);
    let reports = {
        let (mut shooters, mut targets) = state.get_mut(&mut world);
        fire(
            shooter,
            FireOrder {
                mode:         &mode,
                target_cell:  Cell::new(8, 5),
                target_level: Level::new(0),
            },
            &mut shooters,
            &mut targets,
            BattleGrids {
                occupancy: &occupancy,
                surface:   &surface,
                cover:     &cover,
            },
            &tuning,
            &mut r,
        )
    };

    assert!(reports.is_empty(), "an empty magazine must fire nothing");
    // No draw was taken — the used RNG matches a fresh stream's next draw.
    assert_eq!(
        r.next_u64(),
        fresh.next_u64(),
        "no draw on a fail-closed fire"
    );
    // No TU charged — the pool is unchanged.
    let tu = world.get::<Tu>(shooter).copied();
    assert_eq!(
        tu,
        Some(Tu::new(200)),
        "no TU charged on a fail-closed fire"
    );
}

/// AC2 — fail-closed when the shooter is not Alive (Downed): an empty volley,
/// no charge, no draw.
#[test]
fn dead_shooter_fires_nothing() {
    let mut world = World::new();
    let tuning = CombatTuning::default();
    let mode = single_mode(0.2, 3);
    let shooter = spawn_shooter(
        &mut world,
        ShooterSpec {
            x: 5,
            y: 5,
            tu: 200,
            tu_max: 100,
            ammo: 10,
            mode,
            aiming: false,
        },
    );
    // Down the shooter — can_fire requires Alive.
    if let Some(mut life) = world.get_mut::<LifeState>(shooter) {
        *life = LifeState::Downed;
    }

    let occupancy = OccupancyGrid::new();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let mut r = rng();

    let mut state: SystemState<(ShooterQuery, TargetQuery)> = SystemState::new(&mut world);
    let reports = {
        let (mut shooters, mut targets) = state.get_mut(&mut world);
        fire(
            shooter,
            FireOrder {
                mode:         &mode,
                target_cell:  Cell::new(8, 5),
                target_level: Level::new(0),
            },
            &mut shooters,
            &mut targets,
            BattleGrids {
                occupancy: &occupancy,
                surface:   &surface,
                cover:     &cover,
            },
            &tuning,
            &mut r,
        )
    };

    assert!(reports.is_empty(), "a downed shooter must fire nothing");
    let tu = world.get::<Tu>(shooter).copied();
    assert_eq!(tu, Some(Tu::new(200)), "no TU charged for a downed shooter");
    let mag = world.get::<Magazine>(shooter).map(|m| **m);
    assert_eq!(mag, Some(10), "ammo unchanged for a downed shooter");
}
