use super::support::*;
use crate::resolve_and_apply::WoundRoll;

// A LOW-banded target in a cover cell, in line with the shooter.
fn spawn_inline_target(
    world: &mut World,
    occupancy: &mut OccupancyGrid,
    cover: &mut CoverLedger,
) -> Entity {
    let target = world.spawn(target_bundle(500, 60)).id();
    equip_uniform_armor(world, target, 20, 60, 200, 10);
    let target_at = CellLevel::new(Cell::new(8, 5), Level::new(0));
    occupancy.set_occupant(target_at, Some(target));
    occupancy.set_occupant_band(target_at, Some(HeightBand::Low));
    cover.insert(
        target_at,
        CoverEntry::seeded(
            CoverHp::new(10),
            HeightBand::Low,
            ArmorProtection::new(0),
            ArmorHardness::new(0),
            TerrainPieceKind::Cover,
        ),
    );
    target
}

// Fire one full burst from a fresh world; returns the volley and the target.
fn fire_burst(tuning: &CombatTuning, mode: FireModeSpec) -> (Volley, Entity) {
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
    let mut occupancy = OccupancyGrid::new();
    let mut cover = CoverLedger::new();
    let target = spawn_inline_target(&mut world, &mut occupancy, &mut cover);
    let surface = SurfaceGrid::new();
    let mut slab = SlabLedger::new();
    let mut shot_r = rng();
    let mut sev_r = severity_rng();
    let mut state: SystemState<FireQueries> = SystemState::new(&mut world);
    let volley = match state.get_mut(&mut world) {
        Ok((mut shooters, mut arms, mut bodies)) => fire(
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
                tuning,
                severity_rng: &mut sev_r,
                tables: &injury_tables(),
                registry: &injury_registry(),
                injury_rng: &mut injury_rng(),
            },
        ),
        Err(_) => Volley::empty(),
    };
    (volley, target)
}

#[test]
fn recoil_climbs_across_burst_and_resets_between_calls() {
    let mut tuning = CombatTuning::default();
    tuning.cone_stability.recoil_climb = crate::tuning::RecoilClimb::new(2.0);
    let mode = single_mode(0.1, 3);

    let (volley, target) = fire_burst(&tuning, mode);
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

    let (volley_again, _) = fire_burst(&tuning, mode);
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
