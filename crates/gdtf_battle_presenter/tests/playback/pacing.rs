use std::time::Duration;

use bevy::{ecs::system::RunSystemOnce, prelude::*};
use gdtf_battle_presenter::{Played, ShotProjectile, playback_caught_up};
use gdtf_battle_sim::{
    act_log::{ActDeed, ActLog, ActProvenance, ActSeq},
    ganger::Direction,
    shot_fired::ShotFired,
};

use super::harness::*;

#[test]
fn the_cursor_releases_one_reaction_shot_per_beat() {
    let mut app = playback_app();
    let mover = spawn_ganger(&mut app, ground(4, 4), Direction::North);
    let reactors: Vec<Entity> = (0..5)
        .map(|index| spawn_ganger(&mut app, ground(index, 0), Direction::East))
        .collect();
    seed(&mut app);

    for reactor in &reactors {
        append_reaction_fire(&mut app, *reactor, mover);
    }

    let beat = *tuning(&app).reaction_beat_seconds;
    let increment = Duration::from_secs_f32(beat / 4.0);

    let mut released_per_step: Vec<u64> = Vec::new();
    let mut steps = 0_u32;
    let mut previous = shown(&app);
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

    let elapsed = increment.saturating_mul(steps);
    let summed = Duration::from_secs_f32(beat * 5.0);
    assert!(
        elapsed >= summed,
        "showing five reaction shots must take at least the summed dwell ({summed:?}); took \
         {elapsed:?} over {steps} steps",
    );
}

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

    step(&mut app, Duration::from_millis(1));
    assert_eq!(*shown(&app), 1, "the round is shown");
    assert!(holding(&app), "the cursor holds for the bolt");

    for _ in 0..5 {
        step(&mut app, Duration::from_millis(50));
        assert_eq!(
            *shown(&app),
            1,
            "the cursor must not advance past the round while its bolt has never appeared — \
             an idle pipeline before the bolt spawns is the gap, not a drained pipeline",
        );
    }

    let bolt = app.world_mut().spawn(ShotProjectile).id();
    step(&mut app, Duration::from_millis(50));
    assert_eq!(*shown(&app), 1, "still holding while the bolt is in flight");
    app.world_mut().entity_mut(bolt).despawn();
    for _ in 0..8 {
        step(&mut app, Duration::from_millis(100));
    }
    assert_eq!(
        *shown(&app),
        2,
        "once the bolt has flown and landed, the entry behind it is shown",
    );
}

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

    app.world_mut().spawn(ShotProjectile);

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

const _: fn() -> Option<Played<ShotFired>> = || None;

const _START: ActSeq = ActSeq::START;
