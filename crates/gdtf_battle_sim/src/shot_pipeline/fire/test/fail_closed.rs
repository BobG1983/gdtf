use super::support::*;

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
    let mut cover = CoverLedger::new();
    let mut slab = SlabLedger::new();
    let mut shot_r = rng();
    let mut fresh_shot = rng();
    let mut sev_r = severity_rng();

    let mut state: SystemState<FireQueries> = SystemState::new(&mut world);
    let volley = {
        let Ok((mut shooters, mut targets, wears, mut pieces, wields, mut weapons, melee, mounted)) =
            state.get_mut(&mut world)
        else {
            return;
        };
        fire(
            shooter,
            FireOrder {
                mode:         &mode,
                target_cell:  Cell::new(8, 5),
                target_level: Level::new(0),
            },
            &mut shooters,
            &mut targets,
            &wears,
            &mut pieces,
            &wields,
            &mut weapons,
            &melee,
            &mounted,
            BattleGrids {
                occupancy:   &occupancy,
                surface:     &surface,
                cover:       &mut cover,
                slab:        &mut slab,
                brace_cells: &BraceStairCells::empty(),
            },
            &tuning,
            &mut shot_r,
            &mut sev_r,
            &injury_tables(),
            &injury_registry(),
            &mut injury_rng(),
        )
    };

    assert!(
        volley.reports.is_empty() && volley.shots.is_empty(),
        "an empty magazine must fire nothing",
    );
    assert_eq!(
        shot_r.next_u64(),
        fresh_shot.next_u64(),
        "no draw on a fail-closed fire"
    );
    let tu = world.get::<Tu>(shooter).copied();
    assert_eq!(
        tu,
        Some(Tu::new(200)),
        "no TU charged on a fail-closed fire"
    );
}

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
    if let Some(mut life) = world.get_mut::<LifeState>(shooter) {
        *life = LifeState::Downed;
    }

    let occupancy = OccupancyGrid::new();
    let surface = SurfaceGrid::new();
    let mut cover = CoverLedger::new();
    let mut slab = SlabLedger::new();
    let mut shot_r = rng();
    let mut sev_r = severity_rng();

    let mut state: SystemState<FireQueries> = SystemState::new(&mut world);
    let volley = {
        let Ok((mut shooters, mut targets, wears, mut pieces, wields, mut weapons, melee, mounted)) =
            state.get_mut(&mut world)
        else {
            return;
        };
        fire(
            shooter,
            FireOrder {
                mode:         &mode,
                target_cell:  Cell::new(8, 5),
                target_level: Level::new(0),
            },
            &mut shooters,
            &mut targets,
            &wears,
            &mut pieces,
            &wields,
            &mut weapons,
            &melee,
            &mounted,
            BattleGrids {
                occupancy:   &occupancy,
                surface:     &surface,
                cover:       &mut cover,
                slab:        &mut slab,
                brace_cells: &BraceStairCells::empty(),
            },
            &tuning,
            &mut shot_r,
            &mut sev_r,
            &injury_tables(),
            &injury_registry(),
            &mut injury_rng(),
        )
    };

    assert!(
        volley.reports.is_empty() && volley.shots.is_empty(),
        "a downed shooter must fire nothing",
    );
    let tu = world.get::<Tu>(shooter).copied();
    assert_eq!(tu, Some(Tu::new(200)), "no TU charged for a downed shooter");
    let mag = weapon_rounds(&world, shooter);
    assert_eq!(mag, Some(10), "ammo unchanged for a downed shooter");
}
