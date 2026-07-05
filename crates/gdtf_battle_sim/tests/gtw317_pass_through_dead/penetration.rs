//! Rounds pass THROUGH the newly-dead and pre-existing corpses without re-wounding
//! them, byte-equal under the same seed (AC 1-4).

use bevy::prelude::World;
use gdtf_battle_sim::{InflictedWounds, LifeState, OccupancyGrid, Wounds};

use super::harness::*;

/// AC #1 — a BURST whose round 1 KILLS the front ganger passes round 2+ THROUGH the
/// fresh corpse onto a LIVE ganger directly behind it.
///
/// The front ganger spawns with `Wounds = 1`, so any non-graze severity on round 1
/// saturates its life pool to `0` → `LifeState::Dead` BEFORE round 2 runs. We search
/// a small seed set for a seed whose round-1 report actually killed the front (a
/// graze on every seed would not), then assert that on that same seed the volley
/// also struck the BEHIND ganger — i.e. a later round passed through the corpse.
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
        // Front: 1 Wound, so a single non-graze round 1 kills it.
        let front = line_ganger(&mut world, front_cell(), 1, LifeState::Alive);
        // Behind: healthy and alive, directly behind on the same ray.
        let behind = line_ganger(&mut world, behind_cell(), 6, LifeState::Alive);

        let mut occupancy = OccupancyGrid::new();
        place_occupant(&mut occupancy, front_cell(), front);
        place_occupant(&mut occupancy, behind_cell(), behind);

        let behind_wounds_before = world.get::<Wounds>(behind).map_or(0, |w| **w);

        let volley = fire_volley(&mut world, shooter, mode, &occupancy, seed);

        // Did round 1 actually kill the front? (Its first report on the front carries
        // life_after == Dead.) Only then is this seed a valid pass-through witness.
        let front_died_round_one =
            applied_on(&volley, front).is_some_and(|a| a.life_after == LifeState::Dead);
        if !front_died_round_one {
            continue;
        }

        // The front is Dead in the world after the volley.
        let front_life = world.get::<LifeState>(front).copied();
        assert_eq!(
            front_life,
            Some(LifeState::Dead),
            "seed {seed:#x}: the front ganger must be Dead after the killing burst",
        );

        // A later round passed THROUGH the corpse and affected the BEHIND ganger: it
        // is struck in the reports AND its wounds dropped (or it was struck at all).
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

/// AC #2 — a BURST that kills the front ganger with NOTHING behind it: the leftover
/// rounds strike cover / wall / nothing, and the dead front ganger takes NO further
/// wounds (it is struck exactly once — the killing round — never again).
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
        // Nothing behind: no occupant past the front cell.

        let volley = fire_volley(&mut world, shooter, mode, &occupancy, seed);

        let front_died_round_one =
            applied_on(&volley, front).is_some_and(|a| a.life_after == LifeState::Dead);
        if !front_died_round_one {
            continue;
        }

        // The corpse is struck EXACTLY ONCE — the killing round. Every later round
        // passes through it (nothing behind → cover/wall/nothing), never re-wounding
        // the corpse. (resolve_and_apply's corpse-skip would no-op a corpse strike
        // anyway, but the march no longer even reports the corpse after death.)
        let front_strikes = struck_count(&volley, front);
        assert_eq!(
            front_strikes, 1,
            "seed {seed:#x}: the dead front ganger must be struck exactly once (the killing \
             round), never re-wounded — got {front_strikes} strikes in {:?}",
            volley.reports,
        );

        // Exactly one InflictedWound was recorded on the corpse (the killing round) —
        // no further wounds accreted.
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

/// AC #3 — a PRE-EXISTING corpse (spawned `LifeState::Dead`) in front of a live
/// target: a single shot passes THROUGH the corpse to the live target behind it.
#[test]
fn single_shot_passes_through_preexisting_corpse_to_live_target() {
    let mut world = World::new();
    let mode = burst_mode(1); // a single round
    let shooter = spawn_shooter(&mut world, mode);
    // Front: spawned already DEAD (a pre-existing corpse), Wounds already 0.
    let corpse = line_ganger(&mut world, front_cell(), 0, LifeState::Dead);
    // Behind: a live target directly behind the corpse on the same ray.
    let live = line_ganger(&mut world, behind_cell(), 6, LifeState::Alive);

    let mut occupancy = OccupancyGrid::new();
    place_occupant(&mut occupancy, front_cell(), corpse);
    place_occupant(&mut occupancy, behind_cell(), live);

    let live_wounds_before = world.get::<Wounds>(live).map_or(0, |w| **w);

    // A tight cone + braced shooter aimed straight down the line: the one round flies
    // through the corpse onto the live target. (Seed pinned for a deterministic run.)
    let volley = fire_volley(&mut world, shooter, mode, &occupancy, 0xC0FF_EE17);

    assert_eq!(volley.reports.len(), 1, "exactly one round fired");
    // The single round did NOT stop on the corpse.
    assert!(
        !report_struck(&volley, corpse),
        "the single round must NOT strike the pre-existing corpse — got {:?}",
        volley.reports,
    );
    // It passed through and struck the live target behind.
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
    // The corpse took no wound (its record stays empty).
    let corpse_wounds = world.get::<InflictedWounds>(corpse).map_or(0, |w| w.len());
    assert_eq!(
        corpse_wounds, 0,
        "the pre-existing corpse must record NO wound — the round passed through it",
    );
}

/// AC #4 — DETERMINISM: the same [`BattleSeed`] reproduces a byte-equal [`Volley`]
/// (reports + shots), even with the new corpse-skip predicate active mid-burst.
#[test]
fn same_seed_reproduces_byte_equal_volley_with_corpse_skip() {
    let seed = 0xDEAD_BEEF_u64;

    // Build an identical world + fire an identical 3-round burst twice; the volleys
    // (reports AND shots) must be byte-equal. The front ganger has Wounds = 1 so a
    // killing round mid-burst engages the corpse-skip on the SECOND+ round — proving
    // the skip itself is RNG-free and replay-stable.
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
