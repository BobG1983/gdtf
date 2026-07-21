//! Multi-round volley staggering: per-impact FCT + per-impact `ShotImpactResolved`
//! (GTW-327/328), driven through the GTW-727 playback cursor.
//!
//! GTW-727 C33 retired the `InterShotSeconds` launch-delay these tests used to lean on:
//! `spawn_shot_projectiles` no longer staggers a volley itself, because the act log carries
//! one entry per ROUND and the playback cursor releases at most one of them per frame, each
//! earning a `RoundSeconds` beat. Two pacing mechanisms stacked on one volley is exactly what
//! that clause forbids.
//!
//! The property under test is unchanged and the assertions are unweakened — a multi-round
//! volley's numbers and outcome signals appear ONE PER IMPACT over time, never dumped
//! together on the frame the shots resolve. Only the source of the beat moves: these tests
//! now release each round the way the cursor does — one `Played<ShotFired>` at a time, a beat
//! apart, via [`play`] — and still assert zero pops at the drain frame, then strictly
//! monotonic growth, then exactly one outcome signal per round.

use bevy::app::App;
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

/// The per-cell target of both rounds and the muzzle one cell west of it — a short flight.
const TARGET_CELL: Cell = Cell::new(5, 5);
/// The level both rounds resolve on.
const TARGET_LEVEL: Level = Level::new(0);

/// Builds a connecting ganger-hit [`ShotFired`] for `struck` on the shared cell — a fresh
/// shooter entity per round (the log names it via its [`bevy::ecs::entity::Entity`]).
fn connecting_round(app: &mut App, struck: bevy::ecs::entity::Entity) -> ShotFired {
    let report = ganger_hit_report(
        struck,
        BodyPart::Torso,
        4,
        6,
        Severity::None,
        LifeState::Alive,
    );
    ShotFired {
        shooter:      app.world_mut().spawn_empty().id(),
        muzzle:       SimPos::new(4.0, 5.0, 0.0),
        trajectory:   ShotDir::from_direction(bevy::math::Vec3::new(1.0, 0.0, 0.0)),
        impact_cell:  TARGET_CELL,
        impact_level: TARGET_LEVEL,
        kind:         ShotKind::Ganger(struck),
        damage:       DamageType::Kinetic,
        report:       Some(report),
    }
}

/// GTW-327 (slice 2), retargeted onto the GTW-727 cursor: a MULTI-ROUND volley's
/// floating-combat-text pops appear STAGGERED at each shot's own impact, NOT all at once.
///
/// The cursor releases one round per frame, a beat apart, so each round is handed to the FX
/// pipeline as its own [`Played<ShotFired>`](gdtf_battle_presenter::Played) via [`play`]. Round
/// A is played, its bolt flown to impact, and only THEN is round B played — so at the first
/// drain frame there are zero pops, after A's flight A's pop(s) are up, and after B's later
/// flight the live count STRICTLY GROWS. This is the exact same "shot-by-shot, not all at once"
/// property the old `InterShotSeconds` flight-stagger proved, now sourced from the cursor's
/// one-round-per-frame release instead of a launch delay.
///
/// Each round is a connecting hit (so it classifies to ≥ 1 pop); the assertions check MONOTONIC
/// growth (0 → A's pops → strictly more after B), the robust shape of "shot-by-shot" regardless
/// of how many pops each classified report yields. A final assert proves a pop persists for
/// (most of) its tuned lifetime rather than vanishing on the next frame — the slice-1 lifetime
/// fix still holds here.
#[test]
fn a_multi_round_volley_pops_its_fct_staggered_per_impact() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    // The tuned pop lifetime (read off the resident FxTuning so the test is not pinned to a
    // literal magnitude the user may retune).
    let tuning = app.world().get_resource::<FxTuning>().copied();
    assert!(
        tuning.is_some(),
        "FxTuning must be resident after settle_resources",
    );
    let Some(tuning) = tuning else { return };
    let ttl = std::time::Duration::from_secs_f32(*tuning.fct_ttl_seconds);

    // Two struck gangers on the SAME cell (so their pops would overlap if dumped together),
    // each a connecting hit (so each classifies to >= 1 pop).
    let struck_a = app
        .world_mut()
        .spawn(Position::new(CellLevel::new(TARGET_CELL, TARGET_LEVEL)))
        .id();
    let struck_b = app
        .world_mut()
        .spawn(Position::new(CellLevel::new(TARGET_CELL, TARGET_LEVEL)))
        .id();

    // The cursor plays ONE round per frame. Play round A first.
    let round_a = connecting_round(&mut app, struck_a);
    play(&mut app, round_a);

    // Drain the played round on a zero-delta frame: bolt A spawns at the muzzle, and CRUCIALLY
    // no pop is spawned yet — the bug was dumping every pop on the fire frame.
    fire_with_zero_delta(&mut app);
    assert_eq!(
        fct_pop_count(&mut app),
        0,
        "at the drain frame NO pop may exist — the whole point of the fix is that the numbers \
         do not all appear at once on the round's drain frame",
    );

    // Fly bolt A to its impact (a short flight, one cell at the tuned velocity): A's pop(s) come up.
    let short_step = std::time::Duration::from_millis(30);
    step_app(&mut app, short_step, 4);
    let after_first = fct_pop_count(&mut app);
    assert!(
        after_first >= 1,
        "after the first bolt's flight its FCT pop(s) must be up (got {after_first})",
    );

    // The cursor releases round B a BEAT LATER — play it now, on a later frame. Its pop(s)
    // appear only when ITS bolt lands, so the live count STRICTLY GROWS — the volley read
    // shot-by-shot, never dumped together.
    let round_b = connecting_round(&mut app, struck_b);
    play(&mut app, round_b);
    fire_with_zero_delta(&mut app);
    step_app(&mut app, short_step, 4);
    let after_second = fct_pop_count(&mut app);
    assert!(
        after_second > after_first,
        "after the second round's staggered impact MORE pops must be live than after the first \
         ({after_second} must exceed {after_first}) — the second shot's numbers appeared later",
    );

    // The lifetime fix (slice 1) still holds: a freshly-spawned pop persists across a frame far
    // shorter than its tuned lifetime rather than vanishing immediately.
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

/// GTW-328 (slice A), retargeted onto the GTW-727 cursor: a MULTI-ROUND volley's per-shot
/// `ShotImpactResolved` SIGNALS (the shared per-shot impact moment the combat-text LOG keys its
/// outcome lines off) arrive STAGGERED at each shot's own impact, NOT all at once.
///
/// The exact analogue of the FCT test above, on the LOG's signal: the log builds one
/// shot-outcome line per `ShotImpactResolved`, so this proves a burst's outcome lines appear
/// one-per-impact in cadence rather than dumped together. Each round is handed to the pipeline
/// as its own [`Played<ShotFired>`](gdtf_battle_presenter::Played) a beat apart (the cursor's
/// one-round-per-frame release), and each round's `ShotImpactResolved` fires only when ITS bolt
/// arrives: zero at the drain frame, one after A's flight, two after B's later flight.
#[test]
fn a_multi_round_volley_emits_shot_impact_resolved_staggered_per_impact() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    // Two struck gangers on the SAME cell, each a connecting hit (so each yields an outcome
    // signal). The shooter is named via its Entity in the signal (the log resolves it downstream).
    let struck_a = app
        .world_mut()
        .spawn(Position::new(CellLevel::new(TARGET_CELL, TARGET_LEVEL)))
        .id();
    let struck_b = app
        .world_mut()
        .spawn(Position::new(CellLevel::new(TARGET_CELL, TARGET_LEVEL)))
        .id();

    // The cursor plays ONE round per frame. Play round A first.
    let round_a = connecting_round(&mut app, struck_a);
    play(&mut app, round_a);

    // Drain the played round on a zero-delta frame: bolt A spawns at the muzzle, and CRUCIALLY
    // no impact-resolved signal fires yet — the bug was the LOG dumping every outcome line on
    // THIS drain frame (it used to drain ShotFired directly).
    fire_with_zero_delta(&mut app);
    assert_eq!(
        drain_impacts(&mut app).len(),
        0,
        "at the drain frame NO ShotImpactResolved may fire — the whole point of the fix is that \
         the shot-outcome lines do not all appear at once on the round's drain frame",
    );

    // Fly bolt A to its impact (a short flight): exactly A's impact-resolved signal fires.
    let short_step = std::time::Duration::from_millis(30);
    let after_first = step_counting_impacts(&mut app, short_step, 4);
    assert!(
        after_first >= 1,
        "after the first bolt's flight its ShotImpactResolved must have fired (got {after_first})",
    );

    // The cursor releases round B a BEAT LATER — play it now. Its signal fires only when ITS
    // bolt arrives, so the cumulative count STRICTLY GROWS — the volley resolved shot-by-shot.
    let round_b = connecting_round(&mut app, struck_b);
    play(&mut app, round_b);
    fire_with_zero_delta(&mut app);
    let after_second = after_first + step_counting_impacts(&mut app, short_step, 4);
    assert!(
        after_second > after_first,
        "after the second round's staggered impact MORE ShotImpactResolved must have fired in \
         total ({after_second} must exceed {after_first}) — the second shot's outcome resolved later",
    );

    // Both rounds, exactly once each: a two-round volley resolves two outcome signals across the run.
    assert_eq!(
        after_second, 2,
        "a two-round volley must emit exactly two ShotImpactResolved (one per shot), staggered, \
         got {after_second}",
    );
}
