use bevy::app::App;
use gdtf_battle_sim::{
    acts::MoveRequested, ganger::Direction, resolve_and_apply::HitVerdict, shot_fired::ShotFired,
    test_support::SituationBuilder,
};

use super::harness::*;

type LogFingerprint = Vec<(u64, &'static str, String)>;

type HitFingerprint = Vec<(bool, i32)>;

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

#[derive(bevy::prelude::Resource, Default)]
struct ShotLog(HitFingerprint);

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

fn record_shots(app: &mut App) {
    app.init_resource::<ShotLog>();
    app.add_systems(bevy::app::Update, drain_shots);
}

fn shot_fingerprint(app: &App) -> HitFingerprint {
    app.world()
        .get_resource::<ShotLog>()
        .map(|log| log.0.clone())
        .unwrap_or_default()
}
