//! AC6 — application: an in-line target takes a `Ganger` hit with applied damage,
//! while a fire into empty space yields only `no_effect` reports.

use super::support::*;

/// AC6 — application: firing at an in-line target yields a `HitReport` with
/// `ShotKind::Ganger(entity)` + an `AppliedDamage` block, and the target's
/// Hp/Wounds changed in the world; while a fire into empty space yields only
/// `no_effect` reports and the (absent) target is untouched.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "this integration test drives two full fire() scenarios (hit + miss) with \
              many assertions; the GTW-438 injury-arg threading pushes it one line over \
              the 100 gate — splitting it would obscure the hit-vs-miss comparison"
)]
fn fire_at_in_line_target_applies_damage() {
    let mut world = World::new();
    let tuning = CombatTuning::default();
    let mode = single_mode(0.2, 1);
    // Shooter at (2,5) facing East; aiming for a tight cone onto the target.
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

    // A standing (HIGH-band) target directly East at (8,5,0). fire()'s aim point
    // uses the documented default Standing target stance (the locked target query
    // carries no Stance), so the round flies at the standing silhouette; a HIGH
    // occupant band is equal-or-lower than any round band → it impacts (the march
    // bands the round vs the occupant's published band).
    let target = world.spawn(target_bundle(30, 6)).id();
    // GTW-323: equip the target's worn-armor PIECE entities (the combat read+wear path) —
    // the ONLY armor storage (no on-ganger copy, GTW-323 slice 3).
    equip_uniform_armor(&mut world, target, 0, 0, 1, 0);
    let target_at = CellLevel::new(Cell::new(8, 5), Level::new(0));

    // Build the occupancy grid with the target as a HIGH-band occupant in line.
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
        let Ok((mut shooters, mut targets, wears, mut pieces, wields, mut weapons, melee)) =
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
        report.applied.is_some(),
        "a ganger hit must carry an `AppliedDamage` block",
    );
    // The target's pools changed in the world (the fold mutated in place).
    let hp = world.get::<Hp>(target).map(|h| **h);
    assert!(
        hp.is_some_and(|h| h < 30),
        "the target's Hp must have dropped from 30, got {hp:?}",
    );

    // GTW-279 — the InflictedWounds record the fire path wrote must match the
    // resolution's OWN values (the report it returned), NOT a hand-set fixture:
    // a non-graze hit appends exactly one InflictedWound carrying the rolled tier
    // (report.applied.severity) + the struck part (report.part); a graze appends
    // nothing.
    let Some(applied) = report.applied else {
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
        let Some(part) = report.part else {
            return;
        };
        assert_eq!(
            recorded.as_slice(),
            &[InflictedWound::new(applied.severity, part)],
            "the recorded InflictedWound must carry the resolution's own tier + struck part",
        );
    }
}

/// AC6 (the miss half) — a fire into empty space (no occupant in the path) yields
/// only a `no_effect` report: one round fired, no `AppliedDamage` block, and (the
/// target being absent) nothing is mutated.
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
    let occupancy = OccupancyGrid::new(); // no occupant anywhere
    let surface = SurfaceGrid::new();
    let mut cover = CoverLedger::new();
    let mut slab = SlabLedger::new();
    let mut shot_r = rng();
    let mut sev_r = severity_rng();
    let mut state: SystemState<FireQueries> = SystemState::new(&mut world);
    let volley = {
        let Ok((mut shooters, mut targets, wears, mut pieces, wields, mut weapons, melee)) =
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
    assert_eq!(volley.reports.len(), 1, "one round fired into empty space");
    let Some(report) = volley.reports.first() else {
        return;
    };
    assert_eq!(report.applied, None, "a clean miss applies no damage");
}
