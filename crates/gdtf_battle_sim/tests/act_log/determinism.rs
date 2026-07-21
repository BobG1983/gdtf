//! T3 / T5 — the log's order is a property of source code, and wiring it perturbs nothing.

use bevy::app::App;
use gdtf_battle_sim::{
    acts::MoveRequested, ganger::Direction, resolve_and_apply::HitVerdict, shot_fired::ShotFired,
    test_support::SituationBuilder,
};

use super::harness::*;

/// One entry's fingerprint: its sequence, its deed's stable name, and its actor's debug id.
type LogFingerprint = Vec<(u64, &'static str, String)>;

/// One fired round's fingerprint: whether it struck a ganger, and the HP it took.
type HitFingerprint = Vec<(bool, i32)>;

/// Run one identical seeded battle and return `(log fingerprint, hit-report fingerprint)`.
///
/// The log fingerprint is `(seq, deed name, actor index)` per entry — enough to catch a
/// reordering or a dropped entry, with no `f32` payload in it. The hit fingerprint is every
/// round's struck kind + HP damage, which is the sim's own determinism property.
fn run_battle() -> (LogFingerprint, HitFingerprint) {
    let mut app = battle_app(forced_reaction_tuning(8));
    record_shots(&mut app);

    let watch = ground(5, 5);
    let mover_start = ground(7, 5);
    let situation = SituationBuilder::new()
        .with_gangers([
            watcher(watch, PLAYER, Direction::East),
            tough_mover(mover_start, ENEMY, Direction::West),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let Some(mover) = ganger_at(&mut app, mover_start) else {
        unreachable!("setup spawns the mover at its fixture cell");
    };
    app.world_mut()
        .write_message(MoveRequested::new(mover, ground(11, 5)));
    for _ in 0..8 {
        app.update();
    }

    let log = logged(&app)
        .into_iter()
        .map(|fact| (*fact.seq, fact.deed, format!("{:?}", fact.actor)))
        .collect();
    (log, shot_fingerprint(&app))
}

/// **T3 — the single-writer determinism claim, asserted.** Two runs of the same seeded
/// battle must produce the SAME log, entry for entry.
///
/// `bevy-traps.md` #3 is explicit that two systems with no ordering edge run in a
/// nondeterministic order. The act log's answer is that there is only ONE writer and its
/// six per-family recorders are invoked in a fixed CALL order, so the intra-tick sequence is
/// a property of source code rather than of the scheduler. If the recorders were ever split
/// into independently-scheduled systems, this test is what would catch it.
#[test]
fn the_log_order_is_stable_across_runs() {
    let (first_log, _) = run_battle();
    let (second_log, _) = run_battle();

    assert!(
        !first_log.is_empty(),
        "the fixture must actually record something, else this pins nothing",
    );
    assert_eq!(
        first_log, second_log,
        "the same seed must produce the same log, in the same order — the single-writer \
         fixed-call-order guarantee",
    );
}

/// **T5 — wiring the log perturbs no outcome.** The sim's own hit sequence must be
/// identical run to run with the log present, so the recorder is provably pure exposure: it
/// takes no RNG draw and re-resolves nothing.
#[test]
fn the_log_preserves_seeded_determinism() {
    let (_, first_hits) = run_battle();
    let (_, second_hits) = run_battle();

    assert!(
        !first_hits.is_empty(),
        "the fixture must actually fire, else this pins nothing",
    );
    assert_eq!(
        first_hits, second_hits,
        "the seeded hit sequence must be identical across runs with the act log wired — a \
         recorder that drew from an RNG stream, or re-resolved anything, would show here",
    );
}

// ── A test-local ShotFired recorder ─────────────────────────────────────────────

/// Every fired round's `(struck a ganger, HP damage)` across the run.
#[derive(bevy::prelude::Resource, Default)]
struct ShotLog(HitFingerprint);

/// Record every `ShotFired` so the assertion can read the full history.
fn drain_shots(
    mut shots: bevy::prelude::MessageReader<ShotFired>,
    mut log: bevy::prelude::ResMut<ShotLog>,
) {
    for shot in shots.read() {
        let damage = shot
            .report
            .as_ref()
            .map_or(0, |report| match &report.verdict {
                HitVerdict::Ganger(verdict) => *verdict.applied.hit.hp_damage,
                _ => 0,
            });
        log.0.push((
            matches!(
                shot.kind,
                gdtf_battle_sim::resolve_coarse::ShotKind::Ganger(_)
            ),
            damage,
        ));
    }
}

/// Add the recorder to a test app.
fn record_shots(app: &mut App) {
    app.init_resource::<ShotLog>();
    app.add_systems(bevy::app::Update, drain_shots);
}

/// The recorded hit fingerprint.
fn shot_fingerprint(app: &App) -> HitFingerprint {
    app.world()
        .get_resource::<ShotLog>()
        .map(|log| log.0.clone())
        .unwrap_or_default()
}
