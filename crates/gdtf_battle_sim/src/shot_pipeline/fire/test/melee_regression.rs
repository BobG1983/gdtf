use super::support::*;

#[test]
fn ganger_wielding_both_a_ranged_and_a_melee_weapon_still_fires_the_ranged_weapon() {
    let tuning = CombatTuning::default();
    let mode = single_mode(0.3, 5);

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
            aiming: false,
        },
    );
    equip_melee_weapon(&mut world, shooter);

    let ammo_before = weapon_rounds(&world, shooter);

    let occupancy = OccupancyGrid::new();
    let surface = SurfaceGrid::new();
    let mut cover = CoverLedger::new();
    let mut slab = SlabLedger::new();
    let mut shot_r = rng();
    let mut sev_r = severity_rng();

    let report_count = {
        let mut state: SystemState<FireQueries> = SystemState::new(&mut world);
        let Ok((mut shooters, mut targets, wears, mut pieces, wields, mut weapons, melee, mounted)) =
            state.get_mut(&mut world)
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
        volley.reports.len()
    };

    assert_eq!(
        report_count, 5,
        "the ranged weapon must fire its full 5-round burst despite a melee weapon being wielded \
         (the MeleeWeapon marker excludes the melee entity from the ranged resolution)",
    );

    let ammo_after = {
        use crate::weapon::{MeleeWeapon, Wields};
        let melee_entities: std::collections::HashSet<bevy::prelude::Entity> = world
            .query_filtered::<bevy::prelude::Entity, bevy::prelude::With<MeleeWeapon>>()
            .iter(&world)
            .collect();
        world
            .get::<Wields>(shooter)
            .and_then(|w| w.ranged_weapon(|e| melee_entities.contains(&e)))
            .and_then(|ranged| world.get::<crate::magazine::Magazine>(ranged))
            .map(|m| *m.rounds())
    };
    assert_eq!(
        ammo_before,
        Some(30),
        "precondition: the ranged weapon spawned with 30 rounds",
    );
    assert_eq!(
        ammo_after,
        Some(25),
        "the RANGED magazine dropped by 5 (the burst spent it) — proof fire() resolved the gun, \
         not the magazine-less melee weapon",
    );
}
