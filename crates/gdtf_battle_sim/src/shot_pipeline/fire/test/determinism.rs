//! AC7 — seeded determinism: two same-seed `fire()` runs produce byte-equal volleys.

use super::support::*;

/// AC7 — seeded determinism: two same-seed `fire()` runs over identical worlds
/// produce byte-equal volley reports.
#[test]
fn same_seed_reproduces_byte_equal_volley() {
    let tuning = CombatTuning::default();
    let mode = single_mode(0.15, 4);

    let run = || {
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
        let target = world.spawn(target_bundle(60, 12)).id();
        // GTW-323: equip the target's worn-armor PIECE entities (the combat read+wear
        // path) — the same uniform stats both seeded runs share, so the byte-equal-
        // volley determinism property holds through the keyed piece lookup.
        equip_uniform_armor(&mut world, target, 1, 4, 20, 1);
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
        // `get_mut` now returns a `Result` (Bevy 0.19); these params always validate.
        match state.get_mut(&mut world) {
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
        }
    };

    assert_eq!(
        run(),
        run(),
        "the same battle seed must reproduce the identical volley reports",
    );
}
