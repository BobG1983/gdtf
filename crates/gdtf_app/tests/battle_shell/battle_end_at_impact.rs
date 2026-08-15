//! (CORRECT — it is the "outcome decided" latch). The BUG was that the marker-gated transition
use bevy::{app::App, prelude::*, state::state::State, time::TimeUpdateStrategy};
use gdtf_app::test_support::{AppState, BattleScapeState, RunningState};
use gdtf_battle_presenter::{PendingImpact, Played, ShotProjectile};
use gdtf_battle_sim::{
    armor::BodyPart,
    battle::{BattleLost, BattleWon},
    matchup::Matchup,
    prelude::{Cell, Level, LifeState, SimPos},
    resolve_and_apply::{AppliedDamage, HitReport},
    resolve_coarse::ShotKind,
    resolve_hit::{HitResult, HpDamage, IntegrityWear, PenetratingDamage},
    sample_cone::ShotDir,
    severity::Severity,
    shot_fired::ShotFired,
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until};

/// Fixed steps a frame runs, so the descent advances per frame and never off the real clock.
const ONE_STEP_A_FRAME: u32 = 1;

const FLIGHT_STEP: std::time::Duration = std::time::Duration::from_millis(20);

const FLIGHT_TICKS: u32 = 30;

fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

fn battlescape_state(app: &App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

fn in_battle_running(app: &App) -> bool {
    battlescape_state(app) == Some(BattleScapeState::BattleRunning)
}

fn projectile_count(app: &mut App) -> usize {
    let mut q = app.world_mut().query::<&ShotProjectile>();
    q.iter(app.world()).count()
}

fn pending_impact_count(app: &mut App) -> usize {
    let mut q = app.world_mut().query::<&PendingImpact>();
    q.iter(app.world()).count()
}

fn fx_pipeline_idle(app: &mut App) -> bool {
    projectile_count(app) == 0 && pending_impact_count(app) == 0
}

fn battle_running_app() -> App {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(ONE_STEP_A_FRAME));
    advance_until(&mut app, |app| {
        running_state(app) == Some(RunningState::Menu)
    });
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);

    advance_until(&mut app, in_battle_running);
    app
}

fn lethal_ganger_hit(struck: Entity) -> HitReport {
    HitReport {
        kind:    ShotKind::Ganger(struck),
        verdict: gdtf_battle_sim::resolve_and_apply::HitVerdict::Ganger(Box::new(
            gdtf_battle_sim::resolve_and_apply::GangerVerdict {
                target:      struck,
                part:        BodyPart::Torso,
                applied:     AppliedDamage {
                    matchup:    Matchup::Neutral,
                    hit:        HitResult {
                        penetrating: PenetratingDamage::new(8),
                        hp_damage:   HpDamage::new(12),
                        wear:        IntegrityWear::new(0),
                    },
                    severity:   Severity::Critical,
                    life_after: LifeState::Dead,
                    wear:       gdtf_battle_sim::armor_wear::ArmorWearOutcome::Unaffected,
                },
                injury:      None,
                dot_applied: None,
            },
        )),
    }
}

/// — long enough that, under the BUG, `move_on`'s next `FixedUpdate` tick (a fraction of a second
fn write_deciding_shot(app: &mut App) {
    let cell = Cell::new(55, 5);
    let level = Level::new(0);
    let muzzle = SimPos::new(4.0, 5.0, 0.0);
    let struck = app.world_mut().spawn_empty().id();
    let shooter = app.world_mut().spawn_empty().id();
    let shot = ShotFired {
        shooter,
        muzzle,
        trajectory: ShotDir::from_direction(Vec3::new(1.0, 0.0, 0.0)),
        impact_cell: cell,
        impact_level: level,
        kind: ShotKind::Ganger(struck),
        damage: gdtf_battle_sim::weapon::DamageType::Kinetic,
        report: Some(lethal_ganger_hit(struck)),
    };
    app.world_mut()
        .resource_mut::<Messages<Played<ShotFired>>>()
        .write(Played::new(shot));
}

fn step_app(app: &mut App, step: std::time::Duration, updates: u32) {
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(step));
    for _ in 0..updates {
        app.update();
    }
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::Automatic);
}

fn drain_frame_zero_delta(app: &mut App) {
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::ZERO,
        ));
    app.update();
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::Automatic);
}

#[test]
fn a_shot_kill_keeps_battle_running_until_the_deciding_impact_lands() {
    let mut app = battle_running_app();

    write_deciding_shot(&mut app);
    app.world_mut()
        .resource_mut::<Messages<BattleWon>>()
        .write(BattleWon);
    drain_frame_zero_delta(&mut app);

    assert!(
        projectile_count(&mut app) >= 1,
        "the deciding ShotFired must have spawned a real traveling bolt (the FX pipeline is live)",
    );

    // THE BUG, observed across FixedUpdate ticks WHILE the bolt is mid-flight: advance the clock
    for tick in 0..FLIGHT_TICKS {
        step_app(&mut app, FLIGHT_STEP, 1);
        // (1) THE BUG: the battlescape must STILL be in BattleRunning while the deciding bolt is
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

    step_until(&mut app, std::time::Duration::from_millis(50), |app| {
        battlescape_state(app) == Some(BattleScapeState::AnimateOut)
    });
}

// Advance the virtual clock by `step` per update until `predicate` holds.
fn step_until(app: &mut App, step: std::time::Duration, predicate: impl Fn(&App) -> bool) {
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(step));
    while !predicate(app) {
        app.update();
    }
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::Automatic);
}

#[test]
fn a_flee_with_no_shot_in_flight_ends_promptly() {
    let mut app = battle_running_app();

    assert!(
        fx_pipeline_idle(&mut app),
        "no FX projectile / impact may be in flight before a flee (a non-shot end)",
    );

    app.world_mut()
        .resource_mut::<Messages<BattleLost>>()
        .write(BattleLost);

    advance_until(&mut app, |app| {
        battlescape_state(app) == Some(BattleScapeState::AnimateOut)
    });
}
