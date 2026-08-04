use super::support::*;
use crate::resolve_and_apply::WoundRoll;

#[test]
fn fire_at_in_line_target_applies_damage() {
    let mut world = World::new();
    let tuning = CombatTuning::default();
    let mode = single_mode(0.2, 1);
    let shooter = spawn_shooter(
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

    let target = world.spawn(target_bundle(30, 6)).id();
    equip_uniform_armor(&mut world, target, 0, 0, 1, 0);
    let target_at = CellLevel::new(Cell::new(8, 5), Level::new(0));

    let mut occupancy = OccupancyGrid::new();
    occupancy.set_occupant(target_at, Some(target));
    occupancy.set_occupant_band(target_at, Some(HeightBand::High));
    let surface = SurfaceGrid::new();
    let mut cover = CoverLedger::new();
    let mut slab = SlabLedger::new();
    let mut shot_r = rng();
    let mut sev_r = severity_rng();

    let mut state: SystemState<FireQueries> = SystemState::new(&mut world);
    let volley = {
        let Ok((mut shooters, mut arms, mut bodies)) = state.get_mut(&mut world) else {
            return;
        };
        fire(
            FireOrder {
                shooter,
                mode: &mode,
                target_cell: Cell::new(8, 5),
                target_level: Level::new(0),
            },
            &mut shooters,
            &mut arms,
            &mut bodies,
            BattleGrids {
                occupancy:   &occupancy,
                surface:     &surface,
                cover:       &mut cover,
                slab:        &mut slab,
                brace_cells: &BraceStairCells::empty(),
            },
            &mut shot_r,
            &mut WoundRoll {
                tuning:       &tuning,
                severity_rng: &mut sev_r,
                tables:       &injury_tables(),
                registry:     &injury_registry(),
                injury_rng:   &mut injury_rng(),
            },
        )
    };

    assert_eq!(volley.reports.len(), 1, "one round fired");
    let Some(report) = volley.reports.first() else {
        return;
    };
    assert_eq!(
        report.kind,
        ShotKind::Ganger(target),
        "the in-line shot must strike the target ganger",
    );
    assert!(
        applied_of(report).is_some(),
        "a ganger hit must carry an `AppliedDamage` block",
    );
    let hp = world.get::<Hp>(target).map(|h| **h);
    assert!(
        hp.is_some_and(|h| h < 30),
        "the target's Hp must have dropped from 30, got {hp:?}",
    );

    let Some(applied) = applied_of(report) else {
        return;
    };
    let recorded = world
        .get::<InflictedWounds>(target)
        .map(|w| w.to_vec())
        .unwrap_or_default();
    if applied.severity == Severity::None {
        assert!(
            recorded.is_empty(),
            "a graze (Severity::None) must record NO InflictedWound, got {recorded:?}",
        );
    } else {
        let Some(part) = ganger_verdict(report).map(|v| v.part) else {
            return;
        };
        assert_eq!(
            recorded.as_slice(),
            &[InflictedWound::new(applied.severity, part)],
            "the recorded InflictedWound must carry the resolution's own tier + struck part",
        );
    }
}

#[test]
fn fire_into_empty_space_is_a_clean_miss() {
    let tuning = CombatTuning::default();
    let mode = single_mode(0.2, 1);
    let mut world = World::new();
    let shooter = spawn_shooter(
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
    let occupancy = OccupancyGrid::new();
    let surface = SurfaceGrid::new();
    let mut cover = CoverLedger::new();
    let mut slab = SlabLedger::new();
    let mut shot_r = rng();
    let mut sev_r = severity_rng();
    let mut state: SystemState<FireQueries> = SystemState::new(&mut world);
    let volley = {
        let Ok((mut shooters, mut arms, mut bodies)) = state.get_mut(&mut world) else {
            return;
        };
        fire(
            FireOrder {
                shooter,
                mode: &mode,
                target_cell: Cell::new(40, 5),
                target_level: Level::new(0),
            },
            &mut shooters,
            &mut arms,
            &mut bodies,
            BattleGrids {
                occupancy:   &occupancy,
                surface:     &surface,
                cover:       &mut cover,
                slab:        &mut slab,
                brace_cells: &BraceStairCells::empty(),
            },
            &mut shot_r,
            &mut WoundRoll {
                tuning:       &tuning,
                severity_rng: &mut sev_r,
                tables:       &injury_tables(),
                registry:     &injury_registry(),
                injury_rng:   &mut injury_rng(),
            },
        )
    };
    assert_eq!(volley.reports.len(), 1, "one round fired into empty space");
    let Some(report) = volley.reports.first() else {
        return;
    };
    assert_eq!(
        report.verdict,
        HitVerdict::NoEffect,
        "a clean miss applies no damage"
    );
}
