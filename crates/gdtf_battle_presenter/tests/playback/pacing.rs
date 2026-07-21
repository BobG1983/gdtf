//! T9 / T10 / T11 / T12 — the beat, the two impact-hold traps, and the soft-lock guard.

use std::time::Duration;

use bevy::{ecs::system::RunSystemOnce, prelude::*};
use gdtf_battle_presenter::{Played, ShotProjectile, playback_caught_up};
use gdtf_battle_sim::{
    act_log::{ActDeed, ActLog, ActProvenance, ActSeq},
    ganger::Direction,
    shot_fired::ShotFired,
};

use super::harness::*;

/// **T9 — THE READABLE-PACING PROOF.** Five reaction shots that the sim resolved in one
/// tick must reach the screen ONE AT A TIME, each separated by its own beat.
///
/// This is the ticket's complaint stated as an assertion. Before the cursor every one of
/// these landed on the same frame, which is precisely why "both enemies reaction-fired and
/// you could not tell who was shooting". The test drives the clock in sub-dwell increments
/// and asserts that no two entries are ever released in the same step, and that getting
/// through all five takes at least the summed dwell — so removing the cursor (all five
/// released on the first run) fails it immediately.
///
/// It also carries the intent of the deleted enemy-act cadence test: pacing still exists,
/// it just lives on the view's clock now instead of the sim's tick counter.
#[test]
fn the_cursor_releases_one_reaction_shot_per_beat() {
    let mut app = playback_app();
    let mover = spawn_ganger(&mut app, ground(4, 4), Direction::North);
    let reactors: Vec<Entity> = (0..5)
        .map(|index| spawn_ganger(&mut app, ground(index, 0), Direction::East))
        .collect();
    seed(&mut app);

    // Five reaction declarations, all appended at once — exactly what one sim tick of
    // multiple interrupts produces.
    for reactor in &reactors {
        append_reaction_fire(&mut app, *reactor, mover);
    }

    let beat = *tuning(&app).reaction_beat_seconds;
    // A sub-dwell increment, so a release can only happen because a dwell genuinely
    // elapsed rather than because one huge step skipped past several.
    let increment = Duration::from_secs_f32(beat / 4.0);

    let mut released_per_step: Vec<u64> = Vec::new();
    let mut steps = 0_u32;
    let mut previous = shown(&app);
    // Run until all five are shown AND the last one's own beat has been served, so the
    // measured span covers every dwell rather than only the gaps between releases.
    // Generous cap; the assertion below is on the SHAPE of the releases, not the count.
    while (*shown(&app) < 5 || holding(&app)) && steps < 200 {
        step(&mut app, increment);
        let now = shown(&app);
        released_per_step.push(now.distance_from(previous));
        previous = now;
        steps += 1;
    }

    assert_eq!(
        *shown(&app),
        5,
        "all five reaction shots must eventually be shown",
    );
    assert!(
        released_per_step.iter().all(|released| *released <= 1),
        "no two entries may be released in the same step — five interrupts are five events \
         in a row, never one indistinguishable blur. Releases per step: {released_per_step:?}",
    );

    // The whole sequence took at least the summed dwell — the beats are REAL, not zero.
    let elapsed = increment.saturating_mul(steps);
    let summed = Duration::from_secs_f32(beat * 5.0);
    assert!(
        elapsed >= summed,
        "showing five reaction shots must take at least the summed dwell ({summed:?}); took \
         {elapsed:?} over {steps} steps",
    );
}

/// **T10 — trap A, the pre-spawn gap.** An impact hold must NOT release while the bolt it
/// is waiting for has not materialized yet.
///
/// A projectile is spawned through a deferred scene spawn, so on the frame its message was
/// played it is simply not queryable. A naive "release when no projectile exists" rule
/// reads idle in that gap and releases immediately — showing the shot's consequences before
/// its bolt has even appeared, which is the exact defect this ticket is about. The guard is
/// that idle only counts as drained once busy has been SEEN.
#[test]
fn an_awaiting_impact_hold_does_not_release_in_the_pre_spawn_gap() {
    let mut app = playback_app();
    let shooter = spawn_ganger(&mut app, ground(0, 0), Direction::East);
    seed(&mut app);
    append_round(&mut app, shooter);
    append(
        &mut app,
        shooter,
        ActProvenance::Commanded,
        ActDeed::BleedStarted,
    );

    // Play the round: this is the frame the bolt is asked for, and it does not exist yet.
    step(&mut app, Duration::from_millis(1));
    assert_eq!(*shown(&app), 1, "the round is shown");
    assert!(holding(&app), "the cursor holds for the bolt");

    // Several sub-cap steps with an EMPTY pipeline — the pre-spawn gap. Nothing may be
    // released: the hold has never seen busy, so idle is not a drain.
    for _ in 0..5 {
        step(&mut app, Duration::from_millis(50));
        assert_eq!(
            *shown(&app),
            1,
            "the cursor must not advance past the round while its bolt has never appeared — \
             an idle pipeline before the bolt spawns is the gap, not a drained pipeline",
        );
    }

    // The bolt materializes, then lands. NOW the hold may release.
    let bolt = app.world_mut().spawn(ShotProjectile).id();
    step(&mut app, Duration::from_millis(50));
    assert_eq!(*shown(&app), 1, "still holding while the bolt is in flight");
    app.world_mut().entity_mut(bolt).despawn();
    // Busy → idle releases the wait; the round's own beat then runs down.
    for _ in 0..8 {
        step(&mut app, Duration::from_millis(100));
    }
    assert_eq!(
        *shown(&app),
        2,
        "once the bolt has flown and landed, the entry behind it is shown",
    );
}

/// **T11 — trap B, the soft-lock backstop.** A round whose bolt NEVER spawns must still
/// release, at the cap.
///
/// This is not hypothetical: the projectile spawner legitimately spawns no bolt at all when
/// its effects sheet is missing or too short — it falls back to popping the numbers in
/// place and moves on. Without a hard cap the cursor would wait forever on a bolt that is
/// never coming, with the input gate shut and no in-battle quit key.
#[test]
fn an_awaiting_impact_hold_force_releases_at_the_cap_when_no_bolt_spawns() {
    let mut app = playback_app();
    let shooter = spawn_ganger(&mut app, ground(0, 0), Direction::East);
    seed(&mut app);
    append_round(&mut app, shooter);
    append(
        &mut app,
        shooter,
        ActProvenance::Commanded,
        ActDeed::BleedStarted,
    );

    step(&mut app, Duration::from_millis(1));
    assert_eq!(*shown(&app), 1);

    // Drive well past the cap with an ALWAYS-EMPTY pipeline — no bolt ever spawns.
    let cap = *tuning(&app).impact_cap_seconds;
    let total = Duration::from_secs_f32(cap + 1.0);
    let increment = Duration::from_millis(100);
    let mut elapsed = Duration::ZERO;
    while elapsed < total && *shown(&app) < 2 {
        step(&mut app, increment);
        elapsed = elapsed.saturating_add(increment);
    }

    assert_eq!(
        *shown(&app),
        2,
        "a round whose bolt never spawns must release at the cap — otherwise the cursor, \
         and the input gate with it, is wedged forever",
    );
}

/// **T12 — THE SOFT-LOCK REGRESSION GUARD.** The catch-up predicate must never read the
/// projectile population.
///
/// The tempting extra conjunct — "and no bolt is still in flight" — turns a leaked or
/// zero-velocity projectile into a permanent false, which with input gated shut and no
/// in-battle quit key means the only escape is killing the process. Projectile velocity is
/// a hot-reloadable tuning value, so authoring it as `0` is a one-character way to brick the
/// game. This test spawns a bolt that never advances and asserts the gate still opens.
#[test]
fn caught_up_never_reads_the_projectile_population() {
    let mut app = playback_app();
    let shooter = spawn_ganger(&mut app, ground(0, 0), Direction::East);
    seed(&mut app);
    append(
        &mut app,
        shooter,
        ActProvenance::Commanded,
        ActDeed::BleedStarted,
    );

    // A bolt that will never move and never be despawned — the leaked / zero-velocity
    // projectile the dangerous conjunct would trip on. (The outstanding-arrival half of
    // "busy" is seeded only inside the FX layer, so the reachable half is pinned here; both
    // are read by the same expression.)
    app.world_mut().spawn(ShotProjectile);

    // Drain the log.
    for _ in 0..8 {
        step(&mut app, Duration::from_millis(200));
    }

    let head = {
        let Some(log) = app.world().get_resource::<ActLog>() else {
            unreachable!("the fixture inserts an act log");
        };
        log.head()
    };
    assert_eq!(shown(&app), head, "the cursor drained the log");

    let caught_up = app
        .world_mut()
        .run_system_once(playback_caught_up)
        .unwrap_or(false);
    assert!(
        caught_up,
        "the catch-up predicate must become true once the log is drained, even with a bolt \
         permanently in flight — reading the projectile population here would soft-lock the \
         game on a zero-velocity tuning value or a single leaked bolt",
    );
}

/// A resolved round by `shooter`, with geometry only.
fn append_round(app: &mut App, shooter: Entity) {
    let shot = ShotFired {
        shooter,
        muzzle: gdtf_battle_sim::metric::SimPos::new(0.0, 0.0, 0.0),
        trajectory: gdtf_battle_sim::sample_cone::ShotDir::from_direction(Vec3::new(1.0, 0.0, 0.0)),
        impact_cell: gdtf_battle_sim::metric::Cell::new(4, 0),
        impact_level: gdtf_battle_sim::metric::Level::new(0),
        kind: gdtf_battle_sim::resolve_coarse::ShotKind::Miss,
        damage: gdtf_battle_sim::weapon::DamageType::Kinetic,
        report: None,
    };
    append(
        app,
        shooter,
        ActProvenance::Commanded,
        ActDeed::RoundResolved {
            shot: Box::new(shot),
        },
    );
}

/// The `Played<ShotFired>` buffer is what the projectile spawner reads — named here so the
/// suite documents the chain a round entry drives.
const _: fn() -> Option<Played<ShotFired>> = || None;

/// The starting sequence, named so a reader can see the cursor begins at zero.
const _START: ActSeq = ActSeq::START;
