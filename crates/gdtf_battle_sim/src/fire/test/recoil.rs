//! AC5 — recoil climbs across the burst (per-round `prior_shots = i`) and resets
//! between `fire()` calls, proven on the real `fire()` path.

use super::support::*;

/// AC5 — recoil climbs across the burst (per-round `prior_shots = i`) and RESETS
/// between `fire()` calls — proven ON THE REAL `fire()` PATH (it CALLS `fire()`),
/// not by re-composing the cone with a helper.
///
/// `prior_shots` is load-bearing on `fire()`'s real output twice over: it widens
/// `cone_for` AND tilts the `climb_aim_dir` central axis upward. Here the weapon's
/// `BaseSpread` is `0`, so the cone is exactly `0` every round and the ONLY thing
/// that moves a round is the climb tilt. A LOW-band target sits in line; the aim
/// point is pinned LOW (the target cell carries a LOW cover band), so round 0
/// (zero tilt) flies level and IMPACTS the LOW occupant. With a deliberately large
/// `recoil_climb`, the later rounds' axis tilts up enough to sail OVER the LOW
/// occupant — so the volley is NOT all-`Ganger`. A regression that hardcoded
/// `PriorShots::new(0)` every round would fly every round level and strike the
/// target on EVERY round (a uniform all-`Ganger` volley), so it FAILS this test.
///
/// RESET: a SECOND `fire()` call over an identical fresh world + identical fresh
/// seed must reproduce the SAME round-0 outcome (and the same whole volley),
/// proving each call starts at `prior_shots = 0` — observed across two real
/// `fire()` invocations, not by re-evaluating one pure closure at arg 0 twice.
#[test]
fn recoil_climbs_across_burst_and_resets_between_calls() {
    // A large recoil_climb so the per-round upward tilt is unmistakable — tests
    // are free to author arbitrary tuning (the central_axis.rs precedent).
    let mut tuning = CombatTuning::default();
    tuning.cone_stability.recoil_climb = crate::tuning::RecoilClimb::new(2.0);
    let mode = single_mode(0.1, 3); // a 3-round burst

    // Build an identical world + a LOW-band in-line target each call, fire a
    // 3-round burst at it, and return the frozen volley.
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
        // A LOW-band ganger occupant directly East at (8,5,0) — deep HP/Wounds and
        // real armor so round 0 wounds but does NOT kill it. That way the volley's
        // later rounds turning non-`Ganger` is unambiguously the recoil climb
        // (they sail OVER), not corpse-skip turning a struck round into no-effect.
        let target = world.spawn(target_bundle(500, 60)).id();
        // GTW-323: equip the target's worn-armor PIECE entities (the combat read+wear
        // path) — the ONLY armor storage (no on-ganger copy, GTW-323 slice 3).
        equip_uniform_armor(&mut world, target, 20, 60, 200, 10);
        let target_at = CellLevel::new(Cell::new(8, 5), Level::new(0));
        let mut occupancy = OccupancyGrid::new();
        occupancy.set_occupant(target_at, Some(target));
        occupancy.set_occupant_band(target_at, Some(HeightBand::Low));
        let surface = SurfaceGrid::new();
        // A LOW cover band at the target cell pins fire()'s aim point LOW (the aim
        // point reads the target cell's cover band), so round 0 flies level into
        // the LOW occupant. The cover is LOW too, so a round that sails over the
        // LOW occupant also sails over it — no ambiguity.
        let mut cover = CoverLedger::new();
        cover.insert(
            target_at,
            CoverEntry::seeded(
                CoverHp::new(10),
                HeightBand::Low,
                ArmorProtection::new(0),
                ArmorHardness::new(0),
            ),
        );
        let mut r = rng();
        let mut state: SystemState<FireQueries> = SystemState::new(&mut world);
        // `get_mut` now returns a `Result` (Bevy 0.19); these params always validate.
        let reports = match state.get_mut(&mut world) {
            Ok((mut shooters, mut targets, wears, mut pieces, wields, mut weapons)) => fire(
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
                BattleGrids {
                    occupancy: &occupancy,
                    surface:   &surface,
                    cover:     &mut cover,
                },
                &tuning,
                &mut r,
            ),
            Err(_) => Volley::empty(),
        };
        (reports, target)
    };

    let (volley, target) = run();
    assert_eq!(volley.reports.len(), 3, "the full 3-round burst fired");

    // Round 0 (zero prior shots → level flight) IMPACTS the LOW occupant.
    let Some(first) = volley.reports.first() else {
        return;
    };
    assert_eq!(
        first.kind,
        ShotKind::Ganger(target),
        "round 0 (zero tilt) must strike the in-line LOW target",
    );
    assert!(
        first.applied.is_some(),
        "round 0's ganger hit must carry an AppliedDamage block",
    );

    // The CLIMB is load-bearing: with prior_shots wired per round, the later
    // rounds tilt UP and sail OVER the LOW occupant, so the volley is NOT all
    // `Ganger(target)`. A fixed-PriorShots::new(0) regression would strike the
    // target on every round (a uniform volley) and FAIL this assertion.
    let all_strike_target = volley
        .reports
        .iter()
        .all(|report| report.kind == ShotKind::Ganger(target));
    assert!(
        !all_strike_target,
        "the recoil climb must lift later rounds off the LOW target — a fixed \
         prior_shots=0 would strike every round: {volley:?}",
    );

    // RESET — a second fire() call over an identical fresh world + fresh seed
    // reproduces the SAME round-0 outcome (each call starts at prior_shots = 0).
    let (volley_again, _) = run();
    let Some(first_again) = volley_again.reports.first() else {
        return;
    };
    assert_eq!(
        first, first_again,
        "a second fire() call must reproduce round 0 — recoil resets to prior_shots=0",
    );
    // The whole volley reproduces too (the per-round climb restarts at 0).
    assert_eq!(
        volley, volley_again,
        "a second fire() call must reproduce the whole volley (recoil resets)",
    );
}
