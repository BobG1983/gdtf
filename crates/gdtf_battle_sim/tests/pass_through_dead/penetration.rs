use bevy::prelude::World;
use gdtf_battle_sim::{
    ganger::Wounds,
    inflicted_wound::InflictedWounds,
    prelude::{LifeState, OccupancyGrid},
};

use super::harness::*;

#[test]
fn burst_kills_front_then_passes_through_to_live_behind() {
    let seeds: [u64; 12] = [
        0x5A1C_AC75,
        0x0BAD_F00D,
        0xDEAD_BEEF,
        0xFEED_FACE,
        0x1234_5678,
        0xCAFE_B0BA,
        0x9E37_79B9,
        0xA11C_E5ED,
        0x0000_0001,
        0x7FFF_FFFF,
        0xABCD_1234,
        0x1357_9BDF,
    ];

    let mut proved = false;
    for seed in seeds {
        let mut world = World::new();
        let mode = burst_mode(3);
        let shooter = spawn_shooter(&mut world, mode);
        let front = line_ganger(&mut world, front_cell(), 1, LifeState::Alive);
        let behind = line_ganger(&mut world, behind_cell(), 6, LifeState::Alive);

        let mut occupancy = OccupancyGrid::new();
        place_occupant(&mut occupancy, front_cell(), front);
        place_occupant(&mut occupancy, behind_cell(), behind);

        let behind_wounds_before = world.get::<Wounds>(behind).map_or(0, |w| **w);

        let volley = fire_volley(&mut world, shooter, mode, &occupancy, seed);

        let front_died_round_one =
            applied_on(&volley, front).is_some_and(|a| a.life_after == LifeState::Dead);
        if !front_died_round_one {
            continue;
        }

        let front_life = world.get::<LifeState>(front).copied();
        assert_eq!(
            front_life,
            Some(LifeState::Dead),
            "seed {seed:#x}: the front ganger must be Dead after the killing burst",
        );

        let behind_struck = report_struck(&volley, behind);
        let behind_wounds_after = world.get::<Wounds>(behind).map_or(0, |w| **w);
        assert!(
            behind_struck,
            "seed {seed:#x}: a round must pass THROUGH the corpse and strike the live ganger \
             behind it — got reports {:?}",
            volley.reports,
        );
        assert!(
            behind_wounds_after < behind_wounds_before || applied_on(&volley, behind).is_some(),
            "seed {seed:#x}: the behind ganger must take an effect (wounds drop / applied damage)",
        );
        proved = true;
        break;
    }

    assert!(
        proved,
        "no seed produced a round-1 kill of the front ganger — widen the seed set; the \
         pass-through could not be witnessed",
    );
}

#[test]
fn burst_kills_front_with_nothing_behind_does_not_re_wound_corpse() {
    let seeds: [u64; 12] = [
        0x5A1C_AC75,
        0x0BAD_F00D,
        0xDEAD_BEEF,
        0xFEED_FACE,
        0x1234_5678,
        0xCAFE_B0BA,
        0x9E37_79B9,
        0xA11C_E5ED,
        0x0000_0001,
        0x7FFF_FFFF,
        0xABCD_1234,
        0x1357_9BDF,
    ];

    let mut proved = false;
    for seed in seeds {
        let mut world = World::new();
        let mode = burst_mode(3);
        let shooter = spawn_shooter(&mut world, mode);
        let front = line_ganger(&mut world, front_cell(), 1, LifeState::Alive);

        let mut occupancy = OccupancyGrid::new();
        place_occupant(&mut occupancy, front_cell(), front);

        let volley = fire_volley(&mut world, shooter, mode, &occupancy, seed);

        let front_died_round_one =
            applied_on(&volley, front).is_some_and(|a| a.life_after == LifeState::Dead);
        if !front_died_round_one {
            continue;
        }

        let front_strikes = struck_count(&volley, front);
        assert_eq!(
            front_strikes, 1,
            "seed {seed:#x}: the dead front ganger must be struck exactly once (the killing \
             round), never re-wounded — got {front_strikes} strikes in {:?}",
            volley.reports,
        );

        let recorded = world.get::<InflictedWounds>(front).map_or(0, |w| w.len());
        assert!(
            recorded <= 1,
            "seed {seed:#x}: the corpse must record at most the one killing wound, got {recorded}",
        );
        proved = true;
        break;
    }

    assert!(
        proved,
        "no seed produced a round-1 kill of the lone front ganger — widen the seed set",
    );
}

#[test]
fn single_shot_passes_through_preexisting_corpse_to_live_target() {
    let mut world = World::new();
    let mode = burst_mode(1);
    let shooter = spawn_shooter(&mut world, mode);
    let corpse = line_ganger(&mut world, front_cell(), 0, LifeState::Dead);
    let live = line_ganger(&mut world, behind_cell(), 6, LifeState::Alive);

    let mut occupancy = OccupancyGrid::new();
    place_occupant(&mut occupancy, front_cell(), corpse);
    place_occupant(&mut occupancy, behind_cell(), live);

    let live_wounds_before = world.get::<Wounds>(live).map_or(0, |w| **w);

    let volley = fire_volley(&mut world, shooter, mode, &occupancy, 0xC0FF_EE17);

    assert_eq!(volley.reports.len(), 1, "exactly one round fired");
    assert!(
        !report_struck(&volley, corpse),
        "the single round must NOT strike the pre-existing corpse — got {:?}",
        volley.reports,
    );
    assert!(
        report_struck(&volley, live),
        "the single round must pass THROUGH the corpse and strike the live target — got {:?}",
        volley.reports,
    );
    let live_wounds_after = world.get::<Wounds>(live).map_or(0, |w| **w);
    let applied = applied_on(&volley, live).is_some();
    assert!(
        live_wounds_after < live_wounds_before || applied,
        "the live target behind the corpse must take an effect (wounds drop / applied damage)",
    );
    let corpse_wounds = world.get::<InflictedWounds>(corpse).map_or(0, |w| w.len());
    assert_eq!(
        corpse_wounds, 0,
        "the pre-existing corpse must record NO wound — the round passed through it",
    );
}

#[test]
fn same_seed_reproduces_byte_equal_volley_with_corpse_skip() {
    let seed = 0xDEAD_BEEF_u64;

    let run = || {
        let mut world = World::new();
        let mode = burst_mode(3);
        let shooter = spawn_shooter(&mut world, mode);
        let front = line_ganger(&mut world, front_cell(), 1, LifeState::Alive);
        let behind = line_ganger(&mut world, behind_cell(), 6, LifeState::Alive);
        let mut occupancy = OccupancyGrid::new();
        place_occupant(&mut occupancy, front_cell(), front);
        place_occupant(&mut occupancy, behind_cell(), behind);
        fire_volley(&mut world, shooter, mode, &occupancy, seed)
    };

    assert_eq!(
        run(),
        run(),
        "the same battle seed must reproduce a byte-equal volley (reports + shots) with the \
         corpse-skip predicate active",
    );
}
