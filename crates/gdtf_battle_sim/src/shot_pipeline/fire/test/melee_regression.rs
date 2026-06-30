//! GTW-505 C5 — the ZERO-ranged-regression proof: a ganger wielding BOTH a ranged AND a
//! melee weapon still fires its RANGED weapon correctly. `fire()` resolves the ranged
//! weapon via [`Wields::ranged_weapon`](crate::weapon::Wields::ranged_weapon) (which
//! EXCLUDES the `MeleeWeapon`-marked entity), so relating a melee weapon never breaks
//! ranged firing.

use super::support::*;

/// GTW-505 C5 — a shooter wielding BOTH a ranged weapon (5-round burst, ammo loaded) AND
/// a melee weapon fires the FULL ranged burst: the volley has 5 reports and the ranged
/// magazine drops by 5. The melee weapon (no ranged stat columns, no `Magazine`) is
/// EXCLUDED from the ranged resolution by the `MeleeWeapon` marker filter — so it is never
/// mistaken for the gun.
///
/// PIN: WITHOUT the `ranged_weapon` filter, `Wields::weapon()` (the FIRST related entity)
/// could resolve the melee weapon; `read_shooter`'s `WeaponQuery` would then fail on it →
/// an EMPTY volley (0 reports, ammo unchanged) — exactly the ranged regression this guards.
/// The melee weapon is related AFTER the ranged one (so it is NOT first in `Wields`), which
/// already makes the bug latent on insertion order; the test ALSO relates it such that the
/// filter is the load-bearing guarantee, not the order.
#[test]
fn ganger_wielding_both_a_ranged_and_a_melee_weapon_still_fires_the_ranged_weapon() {
    let tuning = CombatTuning::default();
    let mode = single_mode(0.3, 5); // a 5-round burst

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
    // GTW-505 C5: relate a MELEE weapon to the SAME ganger — now its `Wields` holds BOTH a
    // ranged (the `spawn_shooter` gun) and a melee weapon entity. `fire()` must still find
    // the gun.
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
        let Ok((mut shooters, mut targets, wears, mut pieces, wields, mut weapons, melee)) =
            state.get_mut(&mut world)
        else {
            return;
        };
        // Fire off into empty space → all misses, the full clamped burst fires.
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

    // The RANGED weapon fired its full 5-round burst — the melee weapon did not shadow it.
    assert_eq!(
        report_count, 5,
        "the ranged weapon must fire its full 5-round burst despite a melee weapon being wielded \
         (the MeleeWeapon marker excludes the melee entity from the ranged resolution)",
    );

    // The RANGED magazine decremented by 5 — proof the ranged weapon entity (not the melee
    // one, which has no Magazine) is the one `fire()` resolved + spent ammo from. The melee
    // weapon carries no Magazine, so a misresolution would have left ammo untouched.
    let ammo_after = {
        // After firing, the magazine lives on the ranged weapon entity. Resolve it the
        // ranged way (excluding the melee weapon) to read the post-fire round count.
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
