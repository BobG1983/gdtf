use super::support::*;

#[test]
fn charge_is_taken_once_and_reflects_aiming() {
    let tuning = CombatTuning::default();
    let mode = single_mode(0.3, 5); 

    let hip_charge = mode_tu_cost(&mode, &TuMax::new(100), &Aiming::new(false), &tuning);
    let aim_charge = mode_tu_cost(&mode, &TuMax::new(100), &Aiming::new(true), &tuning);
    assert!(
        *aim_charge > *hip_charge,
        "aiming must cost strictly more TU"
    );

    for (aiming, expected_charge) in [(false, hip_charge), (true, aim_charge)] {
        let mut world = World::new();
        let shooter = spawn_shooter(
            &mut world,
            ShooterSpec {
                x: 5,
                y: 5,
                tu: 200,
                tu_max: 100,
                ammo: 30,
                mode,
                aiming,
            },
        );
        let occupancy = OccupancyGrid::new();
        let surface = SurfaceGrid::new();
        let mut cover = CoverLedger::new();
        let mut slab = SlabLedger::new();
        let mut shot_r = rng();
        let mut sev_r = severity_rng();

        let mut state: SystemState<FireQueries> = SystemState::new(&mut world);
        {
            let Ok((
                mut shooters,
                mut targets,
                wears,
                mut pieces,
                wields,
                mut weapons,
                melee,
                mounted,
            )) = state.get_mut(&mut world)
            else {
                return;
            };
            let volley = fire(
                shooter,
                FireOrder {
                    mode:         &mode,
                    target_cell:  Cell::new(40, 5),
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
            );
            assert_eq!(volley.reports.len(), 5, "the full 5-round burst fired");
            assert_eq!(
                volley.shots.len(),
                volley.reports.len(),
                "one ShotOutcome per fired round (parallel to the reports)",
            );
        }

        let tu = world.get::<Tu>(shooter).map(|t| **t);
        assert_eq!(
            tu,
            Some(200u8.saturating_sub(*expected_charge)),
            "Tu must drop by exactly ONE mode charge across the whole burst (aiming={aiming})",
        );
    }
}

#[test]
fn ammo_clamps_the_burst_and_drains_the_magazine() {
    let mut world = World::new();
    let tuning = CombatTuning::default();
    let mode = single_mode(0.1, 8); 
    let shooter = spawn_shooter(
        &mut world,
        ShooterSpec {
            x: 5,
            y: 5,
            tu: 200,
            tu_max: 100,
            ammo: 3, 
            mode,
            aiming: false,
        },
    );

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
                target_cell:  Cell::new(40, 5),
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

    assert_eq!(
        volley.reports.len(),
        3,
        "the burst is clamped to the 3 loaded rounds"
    );
    let mag = weapon_rounds(&world, shooter);
    assert_eq!(mag, Some(0), "the magazine ends drained to 0");
}
