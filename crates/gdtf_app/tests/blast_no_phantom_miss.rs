//! GTW-559 — a grenade BLAST detonation must add NO phantom "Someone missed" combat-log line.
//!
//! The bug chain: the presenter's blast seed (`PendingImpact::for_blast`) carries a PLACEHOLDER
//! shooter + a `None` report; `animate_impact` emits its `ShotImpactResolved` unconditionally;
//! the app's combat-log reader resolves the placeholder shooter to the `"Someone"` fallback; and
//! the shared classifier rendered a `None` report as a MISS line — so every detonation appended
//! a phantom `"Someone missed"` line, contradicting the presenter docs' "no phantom shot line"
//! claim. A blast has NO single ganger-shot verdict (its numbers ride the per-ganger wound /
//! injury signals), so no hit-OR-miss outcome line exists for it.
//!
//! This test drives a REAL detonation end-to-end — no stubbed or shadowed log reader:
//! `GdtfLoadTestAppBuilder` (`DefaultPlugins`, a live `AssetServer` rooted at the workspace
//! `assets/`, the whole battlescape incl. the presenter FX pipeline and the combat log) descends
//! to `BattleScapeState::BattleRunning`; a hand-armed thrower's buffered `ThrowGrenadeRequested`
//! (the same message the input seam writes — `bevy-traps.md` #7 carve-out (a)) is re-gated and
//! resolved by the sim's `dispatch_throw_grenade` (`march_arc` + `resolve_blast`, a REAL blast),
//! whose `ThrowResolved` the presenter turns into the shared impact seam's `ShotImpactResolved`,
//! which the app's combat log drains into rendered lines. The assertion: the rendered log holds
//! NO `"Someone missed"` line for the blast. Only the blast's placeholder shooter can resolve to
//! `"Someone"` (every real battle ganger carries a `GangerName`), so that exact text is the
//! phantom's signature — a REAL ganger-shot miss still logs `"<name> missed"` and stays covered
//! by the classifier unit tests.

use bevy::prelude::*;
use gdtf_app::test_support::{AppState, BattleScapeState, CombatLogLine, RunningState};
use gdtf_battle_presenter::ShotImpactResolved;
use gdtf_battle_sim::{
    Cell, CellLevel, Level, Position,
    acts::ThrowGrenadeRequested,
    ganger::{Luck, Tu},
    magazine::{Magazine, ReloadTu},
    test_support::test_weapon_spec,
    weapon::{
        Accuracy, BaseSpread, BlastRadius, DamageType, FatalBias, FireMode, FireModeSpec, HitType,
        Kickback, MagazineSize, ModeConeMult, ModeKind, ModeShots, ModeTuPercent, TrajectoryStyle,
        WeaponDamage, WeaponName, WeaponPunch, WeaponSpec, WieldedBy,
    },
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, MessageProbe, MessageProbePlugin, advance_until};

/// A generous budget for the real `DefaultPlugins` async asset loads + the full state descent
/// under contention (the `battle_end_at_impact.rs` precedent).
const BUDGET: u32 = 512;

/// The update budget for the detonation chain (sim dispatch → `ThrowResolved` → blast seed →
/// `animate_impact` → `ShotImpactResolved`) — a handful of frames, bounded so a dead chain
/// fails instead of hanging.
const CHAIN_BUDGET: u32 = 64;

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

/// Drives the REAL menu → Load → battle path to `BattleScapeState::BattleRunning`, where the
/// presenter's FX impact pipeline AND the combat log are live. Returns the app rested at the
/// live battle (the caller asserts the descent reached it).
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

    if !advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
        BUDGET,
    ) {
        return None;
    }
    Some(app)
}

/// An ARC grenade spec (the `grenade_arc_throw` recipe): `trajectory: Arc` + one blast-shaped
/// `Single` mode, so the sim's throw dispatch re-gates it as a throwable and fans a REAL
/// `HitType::Blast` at the arc's landing.
fn grenade_spec() -> WeaponSpec {
    WeaponSpec {
        base_spread: BaseSpread::new(0.2),
        accuracy: Accuracy::new(0.8),
        kickback: Kickback::new(0.0),
        fatal_bias: FatalBias::new(2.0),
        damage: WeaponDamage::new(20),
        punch: WeaponPunch::new(30),
        damage_type: DamageType::Blast,
        magazine: Magazine::loaded(MagazineSize::new(4), ReloadTu::new(18)),
        fire_mode: FireMode::new(vec![FireModeSpec::with_hit_type(
            ModeKind::Single,
            ModeConeMult::new(1.0),
            ModeTuPercent::new(0.35),
            ModeShots::new(1),
            HitType::Blast {
                radius: BlastRadius::new(1),
            },
        )]),
        trajectory: TrajectoryStyle::Arc,
        ..test_weapon_spec()
    }
}

/// Spawns a hand-armed thrower into the live battle — a bare actor carrying exactly the
/// `(Position, Luck, Tu)` the sim's `dispatch_throw_grenade` gate reads, wielding a spawned
/// Arc-grenade weapon entity (the `WieldedBy` relationship populates its `Wields`). No
/// `Faction` / `LifeState`, so the AI / census never engages it — the throw is the only act.
fn spawn_armed_thrower(app: &mut App, at: CellLevel) -> Entity {
    let thrower = app
        .world_mut()
        .spawn((Position::new(at), Luck::new(10.0), Tu::new(250)))
        .id();
    let (bundle, _siblings) =
        grenade_spec().into_bundle(WeaponName::new("gtw559-test-grenade".to_owned()));
    app.world_mut().spawn((bundle, WieldedBy::new(thrower)));
    thrower
}

/// Adds the impact probe — the generic GTW-576 `MessageProbePlugin<M>` proving the REAL
/// detonation rode the presenter impact pipeline into the combat log's drain (its own
/// `MessageReader` cursor is independent of the log's, so both observe every signal).
fn add_impact_probe(app: &mut App) {
    app.add_plugins(MessageProbePlugin::<ShotImpactResolved>::default());
}

/// Whether the probe has seen the BLAST's impact signal — the placeholder-shooter, `None`-report
/// `ShotImpactResolved` only `PendingImpact::for_blast` produces (every real volley round carries
/// `Some(report)`, even a clean miss).
fn blast_signal_seen(app: &App) -> bool {
    app.world()
        .get_resource::<MessageProbe<ShotImpactResolved>>()
        .is_some_and(|probe| {
            probe
                .seen()
                .iter()
                .any(|impact| impact.report.is_none() && impact.shooter == Entity::PLACEHOLDER)
        })
}

/// The rendered `Text` strings of every combat-log line.
fn log_line_texts(app: &mut App) -> Vec<String> {
    let entities: Vec<Entity> = {
        let mut q = app
            .world_mut()
            .query_filtered::<Entity, With<CombatLogLine>>();
        q.iter(app.world()).collect()
    };
    entities
        .into_iter()
        .filter_map(|e| app.world().get::<Text>(e).map(|t| t.as_str().to_owned()))
        .collect()
}

/// GTW-559 — a REAL grenade detonation, driven through the sim (`ThrowGrenadeRequested` →
/// `dispatch_throw_grenade` → `resolve_blast` → `ThrowResolved`) and the presenter's shared
/// impact seam into the live combat log, renders NO `"Someone missed"` line.
///
/// PIN-DISCRIMINATING: before the fix, the blast's placeholder-shooter / `None`-report
/// `ShotImpactResolved` classified as a miss and the log rendered the phantom
/// `"Someone missed"` — this test's final assertion fails on exactly that line. The probe
/// assertion keeps the test honest post-fix: the blast's signal must still be OBSERVED riding
/// the pipeline (the chain is alive), it just must yield no log line.
#[test]
fn a_blast_detonation_renders_no_phantom_someone_missed_line() {
    let app_opt = battle_running_app();
    assert!(
        app_opt.is_some(),
        "the real Load + descent must reach BattleScapeState::BattleRunning",
    );
    let Some(mut app) = app_opt else {
        return;
    };
    add_impact_probe(&mut app);

    // Arm a thrower mid-map and lob a REAL grenade two cells east — a same-level blind lob
    // (never roof-blocked), resolved entirely by the sim's dispatch + blast fold.
    let thrower_at = CellLevel::new(Cell::new(30, 30), Level::new(0));
    let target_at = CellLevel::new(Cell::new(32, 30), Level::new(0));
    let thrower = spawn_armed_thrower(&mut app, thrower_at);
    app.world_mut()
        .write_message(ThrowGrenadeRequested::new(thrower, target_at));

    // The REAL chain must fire: sim dispatch → ThrowResolved → blast seed → animate_impact →
    // the placeholder/None ShotImpactResolved. A dead chain fails HERE, not silently-green.
    let detonated = advance_until(&mut app, blast_signal_seen, CHAIN_BUDGET);
    assert!(
        detonated,
        "the real detonation must ride the presenter impact pipeline (ThrowResolved -> \
         PendingImpact::for_blast -> ShotImpactResolved with a placeholder shooter + None \
         report) within the chain budget",
    );

    // Let the combat log drain the signal and spawn / settle its line entities.
    for _ in 0..4 {
        app.update();
    }

    // THE BUG: the placeholder shooter resolves to the "Someone" fallback and the None report
    // classified as a miss — the phantom line. Only the blast can produce this exact text
    // (every real battle ganger resolves a GangerName), so its absence is blast-specific and
    // immune to a concurrent REAL "<name> missed" line from the live battle's AI.
    let texts = log_line_texts(&mut app);
    assert!(
        !texts.iter().any(|t| t == "Someone missed"),
        "a grenade blast must NOT render a phantom \"Someone missed\" combat-log line (a blast \
         has no ganger-shot verdict — its numbers ride the per-ganger wound/injury signals); \
         rendered log: {texts:?}",
    );
}
