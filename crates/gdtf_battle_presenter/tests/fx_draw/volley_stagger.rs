//! Multi-round volley staggering: per-impact FCT + per-impact `ShotImpactResolved`
//! (GTW-327/328).

use bevy::{app::App, ecs::message::Messages};
use gdtf_battle_presenter::FxTuning;
use gdtf_battle_sim::{
    armor::BodyPart,
    prelude::{BattleInProgress, Cell, CellLevel, Level, LifeState, Position, SimPos},
    resolve_coarse::ShotKind,
    sample_cone::ShotDir,
    severity::Severity,
    shot_fired::ShotFired,
    weapon::DamageType,
};

use super::{harness::*, probes::*};

/// GTW-327 (slice 2) — the BUG FIX, deterministic + headless: a MULTI-ROUND volley's
/// floating-combat-text pops appear STAGGERED at each shot's own impact, NOT all at once on the
/// drain frame. Two rounds (each a connecting ganger hit) are written in one frame, the same way
/// a burst / full-auto fires; their tracers fly staggered by `InterShotSeconds` (GTW-308), and
/// each shot's pops are spawned only when THAT shot's bolt arrives — so at t=0 there are zero
/// pops, after the first shot's (short) flight the first shot's pops are up, and only ~one
/// `InterShotSeconds` later (the second bolt's launch delay) do the second shot's pops appear.
///
/// Each round is a connecting hit (so it classifies to ≥ 1 pop); the assertions check MONOTONIC
/// growth at the staggered times (0 → first shot's pops → strictly more after the second), which
/// is the robust shape of "shot-by-shot, not all at once" regardless of how many pops each
/// classified report yields. A final assert proves a pop persists for (most of) its tuned
/// lifetime rather than vanishing on the next frame — the slice-1 lifetime fix still holds here.
#[test]
fn a_multi_round_volley_pops_its_fct_staggered_per_impact() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    // The hot-reloadable stagger step + pop lifetime the system uses (read off the resident
    // FxTuning so the test is not pinned to a literal magnitude the user may retune).
    let tuning = app.world().get_resource::<FxTuning>().copied();
    assert!(
        tuning.is_some(),
        "FxTuning must be resident after settle_resources",
    );
    let Some(tuning) = tuning else { return };
    let inter_shot = std::time::Duration::from_secs_f32(*tuning.inter_shot_seconds);
    let ttl = std::time::Duration::from_secs_f32(*tuning.fct_ttl_seconds);
    // Sanity: the stagger gap must exceed the pop lifetime check granularity — the default
    // InterShotSeconds (0.35s) is well above the per-step deltas below.
    assert!(
        inter_shot >= std::time::Duration::from_millis(100),
        "this test assumes a stagger step (InterShotSeconds {inter_shot:?}) comfortably larger \
         than a flight step — the shipped default is 0.35s",
    );

    // Two struck gangers on the SAME cell (so their pops would overlap if dumped together), each
    // a connecting hit (so each classifies to >= 1 pop — the count grows when each shot lands).
    let cell = Cell::new(5, 5);
    let level = Level::new(0);
    let muzzle = SimPos::new(4.0, 5.0, 0.0); // one cell west of the target — a short flight.
    let write_round = |app: &mut App, struck: bevy::ecs::entity::Entity| {
        let report = ganger_hit_report(
            struck,
            BodyPart::Torso,
            4,
            6,
            Severity::None,
            LifeState::Alive,
        );
        let shot = ShotFired {
            shooter: app.world_mut().spawn_empty().id(),
            muzzle,
            trajectory: ShotDir::from_direction(bevy::math::Vec3::new(1.0, 0.0, 0.0)),
            impact_cell: cell,
            impact_level: level,
            kind: ShotKind::Ganger(struck),
            damage: DamageType::Kinetic,
            report: Some(report),
        };
        app.world_mut()
            .resource_mut::<Messages<ShotFired>>()
            .write(shot);
    };
    let struck_a = app
        .world_mut()
        .spawn(Position::new(CellLevel::new(cell, level)))
        .id();
    let struck_b = app
        .world_mut()
        .spawn(Position::new(CellLevel::new(cell, level)))
        .id();
    write_round(&mut app, struck_a);
    write_round(&mut app, struck_b);

    // Drain both ShotFired on a zero-delta frame: BOTH bolts spawn (held at the muzzle), and
    // CRUCIALLY no pop is spawned yet — the bug was dumping every pop here.
    fire_with_zero_delta(&mut app);
    assert_eq!(
        fct_pop_count(&mut app),
        0,
        "at the drain frame NO pop may exist — the whole point of the fix is that the numbers \
         do not all appear at once on the ShotFired-drain frame",
    );

    // Fly the FIRST bolt to its impact: a short flight (one cell at the tuned velocity), well
    // under one InterShotSeconds. After it, the first shot's pop(s) are up; the second bolt is
    // still parked at the muzzle (its launch delay = one InterShotSeconds has not elapsed).
    let short_step = std::time::Duration::from_millis(30);
    step_app(&mut app, short_step, 4);
    let after_first = fct_pop_count(&mut app);
    assert!(
        after_first >= 1,
        "after the first bolt's flight its FCT pop(s) must be up (got {after_first})",
    );

    // Now advance PAST the second bolt's launch delay (one InterShotSeconds) + its flight: the
    // second shot's pops appear, so the live count STRICTLY GROWS — the volley read shot-by-shot.
    step_app(&mut app, inter_shot, 2);
    let after_second = fct_pop_count(&mut app);
    assert!(
        after_second > after_first,
        "after the second bolt's staggered impact MORE pops must be live than after the first \
         ({after_second} must exceed {after_first}) — the second shot's numbers appeared later",
    );

    // The lifetime fix (slice 1) still holds on this path: a freshly-spawned pop persists across
    // a frame far shorter than its tuned lifetime rather than vanishing immediately. Step a small
    // delta (well under the ttl) and confirm the second shot's pops are still alive.
    assert!(
        ttl >= std::time::Duration::from_millis(500),
        "the tuned FCT lifetime ({ttl:?}) is expected to be at least 0.5s (slice-1 fix)",
    );
    step_app(&mut app, std::time::Duration::from_millis(50), 1);
    assert!(
        fct_pop_count(&mut app) >= after_first,
        "a freshly-impacted pop must persist for its tuned lifetime, not vanish on the next frame",
    );
}

/// GTW-328 (slice A) — the BUG FIX, deterministic + headless: a MULTI-ROUND volley's per-shot
/// `ShotImpactResolved` SIGNALS (the shared per-shot impact moment the combat-text LOG keys its
/// outcome lines off) arrive STAGGERED at each shot's own impact, NOT all at once on the
/// `ShotFired`-drain frame. This is the exact analogue of the GTW-327 FCT staggered test, but on
/// the LOG's signal: the log builds one shot-outcome line per `ShotImpactResolved`, so this proves
/// a burst's outcome lines appear one-per-impact in cadence rather than dumped on the fire frame.
///
/// Two connecting ganger-hit rounds are written in one frame (the way a burst / full-auto fires);
/// their bolts fly staggered by `InterShotSeconds` (GTW-308), and each shot's `ShotImpactResolved`
/// is emitted only when THAT shot's bolt arrives. So at the drain frame ZERO signals exist, after
/// the first shot's (short) flight exactly the first shot's signal has fired, and only ~one
/// `InterShotSeconds` later (the second bolt's launch delay) does the second shot's signal fire —
/// monotonic growth across stepped time, the robust shape of "shot-by-shot, not all at once".
#[test]
fn a_multi_round_volley_emits_shot_impact_resolved_staggered_per_impact() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    // The hot-reloadable stagger step the system uses (read off the resident FxTuning so the test
    // is not pinned to a literal magnitude the user may retune).
    let tuning = app.world().get_resource::<FxTuning>().copied();
    assert!(
        tuning.is_some(),
        "FxTuning must be resident after settle_resources",
    );
    let Some(tuning) = tuning else { return };
    let inter_shot = std::time::Duration::from_secs_f32(*tuning.inter_shot_seconds);
    assert!(
        inter_shot >= std::time::Duration::from_millis(100),
        "this test assumes a stagger step (InterShotSeconds {inter_shot:?}) comfortably larger \
         than a flight step — the shipped default is 0.35s",
    );

    // Two struck gangers on the SAME cell, each a connecting hit (so each yields a shot-outcome
    // signal). The shooter is named via its Entity in the signal (the log resolves it downstream).
    let cell = Cell::new(5, 5);
    let level = Level::new(0);
    let muzzle = SimPos::new(4.0, 5.0, 0.0); // one cell west of the target — a short flight.
    let write_round = |app: &mut App, struck: bevy::ecs::entity::Entity| {
        let report = ganger_hit_report(
            struck,
            BodyPart::Torso,
            4,
            6,
            Severity::None,
            LifeState::Alive,
        );
        let shot = ShotFired {
            shooter: app.world_mut().spawn_empty().id(),
            muzzle,
            trajectory: ShotDir::from_direction(bevy::math::Vec3::new(1.0, 0.0, 0.0)),
            impact_cell: cell,
            impact_level: level,
            kind: ShotKind::Ganger(struck),
            damage: DamageType::Kinetic,
            report: Some(report),
        };
        app.world_mut()
            .resource_mut::<Messages<ShotFired>>()
            .write(shot);
    };
    let struck_a = app
        .world_mut()
        .spawn(Position::new(CellLevel::new(cell, level)))
        .id();
    let struck_b = app
        .world_mut()
        .spawn(Position::new(CellLevel::new(cell, level)))
        .id();
    write_round(&mut app, struck_a);
    write_round(&mut app, struck_b);

    // Drain both ShotFired on a zero-delta frame: BOTH bolts spawn (held at the muzzle), and
    // CRUCIALLY no impact-resolved signal fires yet — the bug was the LOG dumping every outcome
    // line on THIS drain frame (it used to drain ShotFired directly).
    fire_with_zero_delta(&mut app);
    assert_eq!(
        drain_impacts(&mut app).len(),
        0,
        "at the drain frame NO ShotImpactResolved may fire — the whole point of the fix is that \
         the shot-outcome lines do not all appear at once on the ShotFired-drain frame",
    );

    // Fly the FIRST bolt to its impact (a short flight, well under one InterShotSeconds): exactly
    // the first shot's impact-resolved signal fires; the second bolt is still parked at the muzzle.
    let short_step = std::time::Duration::from_millis(30);
    let after_first = step_counting_impacts(&mut app, short_step, 4);
    assert!(
        after_first >= 1,
        "after the first bolt's flight its ShotImpactResolved must have fired (got {after_first})",
    );

    // Advance PAST the second bolt's launch delay (one InterShotSeconds) + its flight: the second
    // shot's signal fires, so the cumulative count STRICTLY GROWS — the volley resolved shot-by-shot.
    let after_second = after_first + step_counting_impacts(&mut app, inter_shot, 2);
    assert!(
        after_second > after_first,
        "after the second bolt's staggered impact MORE ShotImpactResolved must have fired in \
         total ({after_second} must exceed {after_first}) — the second shot's outcome resolved later",
    );

    // Both shots, exactly once each: a two-round volley resolves two outcome signals across the run.
    assert_eq!(
        after_second, 2,
        "a two-round volley must emit exactly two ShotImpactResolved (one per shot), staggered, \
         got {after_second}",
    );
}
