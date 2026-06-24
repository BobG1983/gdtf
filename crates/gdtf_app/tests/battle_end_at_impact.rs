//! GTW-334 — the battle must END after the presenter finishes animating the shot that DECIDES
//! it, not at the sim-drain frame the killing shot resolves.
//!
//! The same drain-vs-impact family as GTW-331 (death-despawn), GTW-327 (FCT), GTW-328 (combat
//! log): the sim's victory census (`check_outcome`, GTW-237) emits `BattleWon` / `BattleLost`
//! the SAME drain frame the killing shot's `ShotFired` is emitted and `fire()` applies its
//! damage. The app-side `end_battle_on_outcome` latches `BattleRunningComplete` that frame
//! (CORRECT — it is the "outcome decided" latch). The BUG was that the marker-gated transition
//! to `AnimateOut` fired the same frame — before the deciding tracer animates. The fix DEFERS
//! only the `BattleRunning → AnimateOut` transition until the FX projectile / impact pipeline
//! for the deciding shot has drained.
//!
//! These drive the REAL stack — `GdtfLoadTestAppBuilder` (`DefaultPlugins`, a live `AssetServer`
//! rooted at the workspace `assets/`, the whole battlescape including the presenter's FX
//! projectile / impact pipeline) down to `BattleScapeState::BattleRunning`, where a real
//! `ShotFired` spawns a real traveling bolt that flies and resolves a real `PendingImpact` →
//! `ShotImpactResolved`. The outcome (`BattleWon` / `BattleLost`) + the deciding `ShotFired` are
//! written DIRECTLY into the world buffers in the TEST BODY (`bevy-traps.md` #7 carve-out (a),
//! the established idiom — `battle_running_driver.rs` writes the outcome the same way,
//! `fx_draw.rs` the shot), mimicking exactly what the sim does on the deciding drain: the
//! killing shot's `ShotFired` and the census' outcome on the one frame. No function here takes
//! `&mut World`.

use bevy::{app::App, prelude::*, state::state::State, time::TimeUpdateStrategy};
use gdtf_app::test_support::{AppState, BattleScapeState, RunningState};
use gdtf_battle_presenter::{PendingImpact, ShotProjectile};
use gdtf_battle_sim::{
    AppliedDamage, BattleLost, BattleWon, BodyPart, Cell, HitReport, HitResult, HpDamage,
    IntegrityWear, Level, LifeState, Matchup, PenetratingDamage, Severity, ShotDir, ShotFired,
    ShotKind, SimPos,
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until};

/// A generous budget for the real `DefaultPlugins` async asset loads + the full state descent
/// under contention (the `real_battle_panel.rs` precedent).
const BUDGET: u32 = 512;

/// The per-tick manual clock step the shot-kill repro flies the deciding bolt with — comfortably
/// larger than a `FixedUpdate` tick (≈15.6 ms at 1/64 s) so `move_on`'s marker-gated tick would
/// fire under the bug, yet small enough that `FLIGHT_TICKS` of them keeps the long bolt airborne.
const FLIGHT_STEP: std::time::Duration = std::time::Duration::from_millis(20);

/// How many `FLIGHT_STEP` ticks the shot-kill repro advances mid-flight (≈0.6 s) — well short of
/// the ≈1.6 s the ≈50-cell bolt needs, so the bolt is still in flight throughout, while every one
/// of these ticks would have transitioned under the bug.
const FLIGHT_TICKS: u32 = 30;

/// Reads the current [`RunningState`] if it is active.
fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

/// Reads the current [`BattleScapeState`] if it is active.
fn battlescape_state(app: &App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

/// Whether the state machine is STILL resting in `BattleScapeState::BattleRunning`.
fn in_battle_running(app: &App) -> bool {
    battlescape_state(app) == Some(BattleScapeState::BattleRunning)
}

/// The number of traveling bolts ([`ShotProjectile`]) currently in flight.
fn projectile_count(app: &mut App) -> usize {
    let mut q = app.world_mut().query::<&ShotProjectile>();
    q.iter(app.world()).count()
}

/// The number of outstanding arrival seeds ([`PendingImpact`]) the impact animator has not yet
/// consumed.
fn pending_impact_count(app: &mut App) -> usize {
    let mut q = app.world_mut().query::<&PendingImpact>();
    q.iter(app.world()).count()
}

/// Whether the deciding-shot FX pipeline is QUIESCENT — no bolt in flight and no arrival seed
/// outstanding. The discriminator the fix waits on before leaving `BattleRunning` (the same
/// state the production gate reads).
fn fx_pipeline_idle(app: &mut App) -> bool {
    projectile_count(app) == 0 && pending_impact_count(app) == 0
}

/// Drives the REAL menu → Load → battle path to `BattleScapeState::BattleRunning`, where the
/// presenter's FX projectile / impact pipeline is live. Returns the app rested at the live
/// battle (the caller asserts the descent reached it).
fn battle_running_app() -> Option<App> {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    if !advance_until(
        &mut app,
        |app| running_state(app) == Some(RunningState::Menu),
        BUDGET,
    ) {
        return None;
    }
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);

    if !advance_until(&mut app, in_battle_running, BUDGET) {
        return None;
    }
    Some(app)
}

/// A lethal ganger-hit [`HitReport`] on `struck` — `life_after == Dead`, the verdict a deciding
/// shot carries (the shot that wins / loses the battle). Mirrors `fx_draw.rs::ganger_hit_report`.
const fn lethal_ganger_hit(struck: Entity) -> HitReport {
    HitReport {
        kind:            ShotKind::Ganger(struck),
        part:            Some(BodyPart::Torso),
        applied:         Some(AppliedDamage {
            matchup:    Matchup::Neutral,
            hit:        HitResult {
                penetrating: PenetratingDamage::new(8),
                hp_damage:   HpDamage::new(12),
                wear:        IntegrityWear::new(0),
            },
            severity:   Severity::Critical,
            life_after: LifeState::Dead,
            broken:     None,
            worn:       None,
        }),
        cover_destroyed: None,
    }
}

/// Writes a deciding `ShotFired` (a lethal ganger hit) at a target cell a LONG flight east of the
/// muzzle, the way the sim emits one on the killing drain. The flight is deliberately long
/// (≈50 cells at the shipped ≈480 px/sec velocity ≈ 1.6s) so it spans MANY `FixedUpdate` ticks
/// — long enough that, under the BUG, `move_on`'s next `FixedUpdate` tick (a fraction of a second
/// after the drain latches the marker) fires while the bolt is still nowhere near its impact, so
/// the repro can observe the premature transition. The struck entity is a freshly spawned
/// stand-in (the FX pipeline only needs an `Entity` for the report's `ShotKind::Ganger`; with no
/// rendered sprite the bolt falls back to flying to the impact cell — which is all this
/// app-timing test needs, a real bolt that flies and resolves).
fn write_deciding_shot(app: &mut App) {
    let cell = Cell::new(55, 5);
    let level = Level::new(0);
    let muzzle = SimPos::new(4.0, 5.0, 0.0); // far west — a ≈50-cell, many-FixedUpdate-tick flight.
    let struck = app.world_mut().spawn_empty().id();
    let shooter = app.world_mut().spawn_empty().id();
    let shot = ShotFired {
        shooter,
        muzzle,
        trajectory: ShotDir::from_direction(Vec3::new(1.0, 0.0, 0.0)),
        impact_cell: cell,
        impact_level: level,
        kind: ShotKind::Ganger(struck),
        damage: gdtf_battle_sim::DamageType::Kinetic,
        report: Some(lethal_ganger_hit(struck)),
    };
    app.world_mut()
        .resource_mut::<Messages<ShotFired>>()
        .write(shot);
}

/// Advances the app a fixed number of `step`-sized manual updates (each advancing the virtual
/// clock by `step`), then restores `Automatic` time — the deterministic way to fly the deciding
/// bolt to its impact across several frames. Mirrors `fx_draw.rs::step_app`.
fn step_app(app: &mut App, step: std::time::Duration, updates: u32) {
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(step));
    for _ in 0..updates {
        app.update();
    }
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::Automatic);
}

/// Runs exactly ONE `update()` with the clock delta pinned to ZERO, then restores `Automatic`
/// time — the deciding-drain frame, made delta-deterministic so the freshly spawned bolt stays
/// parked at the muzzle (no contention-dependent flight). Mirrors `fx_draw.rs::fire_with_zero_delta`.
fn drain_frame_zero_delta(app: &mut App) {
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::ZERO,
        ));
    app.update();
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::Automatic);
}

/// REPRO (a) — SHOT-KILL END: a shot that DECIDES the battle keeps the battlescape in
/// `BattleRunning` at the deciding drain frame and through the killing bolt's flight, and
/// transitions to `AnimateOut` ONLY after the deciding shot's impact has resolved (the FX
/// pipeline drained).
///
/// Drives the REAL FX pipeline: a real `ShotFired` (lethal ganger hit) spawns a real traveling
/// bolt, and `BattleWon` is written the SAME drain frame (the killing drain the census emits on).
/// The assertions, in deciding-drain order:
/// 1. At the drain frame the state is STILL `BattleRunning` (the marker may be latched), and a
///    real bolt is now in flight — the deciding shot's tracer.
/// 2. While the bolt is in flight the state stays `BattleRunning` (the deferral holds the
///    transition until the tracer lands).
/// 3. Once the bolt flies to its impact and `ShotImpactResolved` resolves (the FX pipeline goes
///    idle), the state advances to `AnimateOut`.
///
/// RED before the fix: the marker-gated `move_on` set `AnimateOut` the moment
/// `BattleRunningComplete` latched (the drain frame), so assertion (1) (still `BattleRunning`
/// after the drain) fails immediately — the battle ended before the deciding tracer animated.
#[test]
fn a_shot_kill_keeps_battle_running_until_the_deciding_impact_lands() {
    let app_opt = battle_running_app();
    assert!(
        app_opt.is_some(),
        "the real Load + descent must reach BattleScapeState::BattleRunning",
    );
    let Some(mut app) = app_opt else {
        return;
    };

    // The deciding drain: the killing shot's ShotFired AND the census' BattleWon on the SAME
    // frame, exactly as the sim emits them (fire() applies damage -> Dead -> census -> BattleWon,
    // and the ShotFired for that round, all one drain). A zero-delta update so the freshly
    // spawned bolt stays parked at the muzzle (the `spawn_shot_projectiles` Update system runs
    // regardless of clock delta) and no `FixedUpdate` ticks yet (so `move_on` can't fire on this
    // exact frame — the next ticks are what the bug transitions on, observed below).
    write_deciding_shot(&mut app);
    app.world_mut()
        .resource_mut::<Messages<BattleWon>>()
        .write(BattleWon);
    drain_frame_zero_delta(&mut app);

    // A real bolt is in flight — the deciding shot's tracer the transition must wait on.
    assert!(
        projectile_count(&mut app) >= 1,
        "the deciding ShotFired must have spawned a real traveling bolt (the FX pipeline is live)",
    );

    // THE BUG, observed across FixedUpdate ticks WHILE the bolt is mid-flight: advance the clock
    // a series of small manual steps that EACH tick `FixedUpdate` (≈15.6 ms at 1/64 s), so under
    // the bug `move_on`'s marker-gated tick fires and leaves BattleRunning long before the
    // ≈50-cell bolt (≈1.6 s of flight) can reach its impact. The fix keeps the battlescape in
    // BattleRunning every one of these ticks: the FX pipeline is still busy (a bolt in flight),
    // so the transition is deferred. ~30 × 20 ms ≈ 0.6 s of flight — well short of the ≈1.6 s the
    // bolt needs, so it is still airborne throughout. (The OLD code transitioned on the first
    // tick — assertion (1) below is RED pre-fix.)
    for tick in 0..FLIGHT_TICKS {
        step_app(&mut app, FLIGHT_STEP, 1);
        // (1) THE BUG: the battlescape must STILL be in BattleRunning while the deciding bolt is
        // mid-flight, even as FixedUpdate ticks (the schedule the bug's `move_on` fired on).
        assert!(
            in_battle_running(&app),
            "the battlescape must stay in BattleRunning while the deciding bolt is still in \
             flight (the bug left BattleRunning here, on a FixedUpdate tick after the drain, \
             before the tracer lands); failed on flight tick {tick} of {FLIGHT_TICKS}, state was \
             {:?}",
            battlescape_state(&app),
        );
        assert!(
            !fx_pipeline_idle(&mut app),
            "the FX pipeline must still be busy (a bolt in flight) on flight tick {tick} — the \
             ≈50-cell bolt cannot have reached its impact in ≈{}ms",
            FLIGHT_STEP.as_millis() * u128::from(tick + 1),
        );
    }

    // (2) Fly the bolt the rest of the way to its impact: ShotImpactResolved fires, the FX
    // pipeline drains, and the deferred transition advances BattleRunning -> AnimateOut — at the
    // IMPACT, not at the drain. Driven with MANUAL-delta steps (a headless `Automatic`-time loop
    // advances the virtual clock only by tiny wall-clock deltas, so the ≈1.6 s flight would not
    // complete in a bounded update count); each step is comfortably larger than a `FixedUpdate`
    // tick so `move_on` keeps running.
    let reached_animate_out = step_until(
        &mut app,
        std::time::Duration::from_millis(50),
        BUDGET,
        |app| battlescape_state(app) == Some(BattleScapeState::AnimateOut),
    );
    assert!(
        reached_animate_out,
        "once the deciding shot's impact resolves (the FX pipeline drains), the battlescape must \
         advance BattleRunning -> AnimateOut within the manual-step budget; last state was {:?}",
        battlescape_state(&app),
    );
}

/// Advances the app in `step`-sized manual updates until `predicate` holds or `cap` updates
/// elapse, returning whether the predicate was met. The manual-delta twin of
/// [`advance_until`](gdtf_test_utils::advance_until) — a headless `Automatic`-time loop advances
/// the virtual clock only by tiny wall-clock deltas, so a real-duration FX flight needs a known
/// per-update delta to complete in a bounded count. Restores `Automatic` time on exit.
fn step_until(
    app: &mut App,
    step: std::time::Duration,
    cap: u32,
    predicate: impl Fn(&App) -> bool,
) -> bool {
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(step));
    let mut met = false;
    for _ in 0..cap {
        if predicate(app) {
            met = true;
            break;
        }
        app.update();
    }
    met = met || predicate(app);
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::Automatic);
    met
}

/// REPRO (b) — FLEE / NON-SHOT END must stay PROMPT: an outcome with NO shot in flight (a flee,
/// or any non-projectile end) has no `ProjectileTravel` / `PendingImpact` ever, so it must
/// transition to `AnimateOut` IMMEDIATELY — it must NOT hang waiting for an FX pipeline that
/// never goes busy.
///
/// Drives the REAL stack to `BattleRunning`, writes `BattleLost` with NO `ShotFired` in flight
/// (the FX pipeline is idle), and asserts the transition to `AnimateOut` happens promptly. This
/// guards trap B: the deferral must not be an unconditional "wait until the pipeline goes
/// busy-then-idle" — a never-busy pipeline must transition at once.
#[test]
fn a_flee_with_no_shot_in_flight_ends_promptly() {
    let app_opt = battle_running_app();
    assert!(
        app_opt.is_some(),
        "the real Load + descent must reach BattleScapeState::BattleRunning",
    );
    let Some(mut app) = app_opt else {
        return;
    };

    // Sanity: no bolt / impact is in flight at the flee (the discriminator's "never busy" case).
    assert!(
        fx_pipeline_idle(&mut app),
        "no FX projectile / impact may be in flight before a flee (a non-shot end)",
    );

    // A non-shot end: write BattleLost with NO ShotFired (a flee / a bleed-out census loss).
    app.world_mut()
        .resource_mut::<Messages<BattleLost>>()
        .write(BattleLost);

    // It must transition PROMPTLY — not hang waiting for a never-busy pipeline.
    let reached_animate_out = advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::AnimateOut),
        BUDGET,
    );
    assert!(
        reached_animate_out,
        "a non-shot end (no projectile in flight) must transition BattleRunning -> AnimateOut \
         promptly (the deferral must not hang on a never-busy FX pipeline); last state was {:?}",
        battlescape_state(&app),
    );
}
