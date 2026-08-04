use super::support::*;

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "this integration test builds two full fire() scenarios with many assertions; \
              extracting helpers would obscure the scenario structure more than the length"
)]
fn recoil_climbs_across_burst_and_resets_between_calls() {
    let mut tuning = CombatTuning::default();
    tuning.cone_stability.recoil_climb = crate::tuning::RecoilClimb::new(2.0);
    let mode = single_mode(0.1, 3);

    let run = || {
        let mut world = World::new();
        let shooter = spawn_zero_spread_shooter(
            &mut world,
            ShooterSpec {
                x: 2,
                y: 5,
                tu: 200,
                tu_max: 100,
                ammo: 10,
                mode,
                aiming: true,
            },
        );
        let target = world.spawn(target_bundle(500, 60)).id();
        equip_uniform_armor(&mut world, target, 20, 60, 200, 10);
        let target_at = CellLevel::new(Cell::new(8, 5), Level::new(0));
        let mut occupancy = OccupancyGrid::new();
        occupancy.set_occupant(target_at, Some(target));
        occupancy.set_occupant_band(target_at, Some(HeightBand::Low));
        let surface = SurfaceGrid::new();
        let mut cover = CoverLedger::new();
        let mut slab = SlabLedger::new();
        cover.insert(
            target_at,
            CoverEntry::seeded(
                CoverHp::new(10),
                HeightBand::Low,
                ArmorProtection::new(0),
                ArmorHardness::new(0),
            ),
        );
        let mut shot_r = rng();
        let mut sev_r = severity_rng();
        let mut state: SystemState<FireQueries> = SystemState::new(&mut world);
        let reports = match state.get_mut(&mut world) {
            Ok((
                mut shooters,
                mut targets,
                wears,
                mut pieces,
                wields,
                mut weapons,
                melee,
                mounted,
            )) => fire(
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
            ),
            Err(_) => Volley::empty(),
        };
        (reports, target)
    };

    let (volley, target) = run();
    assert_eq!(volley.reports.len(), 3, "the full 3-round burst fired");

    let Some(first) = volley.reports.first() else {
        return;
    };
    assert_eq!(
        first.kind,
        ShotKind::Ganger(target),
        "round 0 (zero tilt) must strike the in-line LOW target",
    );
    assert!(
        applied_of(first).is_some(),
        "round 0's ganger hit must carry an AppliedDamage block",
    );

    let all_strike_target = volley
        .reports
        .iter()
        .all(|report| report.kind == ShotKind::Ganger(target));
    assert!(
        !all_strike_target,
        "the recoil climb must lift later rounds off the LOW target — a fixed \
         prior_shots=0 would strike every round: {volley:?}",
    );

    let (volley_again, _) = run();
    let Some(first_again) = volley_again.reports.first() else {
        return;
    };
    assert_eq!(
        first, first_again,
        "a second fire() call must reproduce round 0 — recoil resets to prior_shots=0",
    );
    assert_eq!(
        volley, volley_again,
        "a second fire() call must reproduce the whole volley (recoil resets)",
    );
}
