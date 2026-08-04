use gdtf_battle_sim::{
    acts::MeleeRequested,
    ganger::{Direction, Fight},
    melee::opposed_fight,
    rng::{BattleSeed, FightRng},
    test_support::SituationBuilder,
    tuning::{CombatTuning, FightVariance, MeleeTuning, ViewRange},
};

use super::harness::*;

fn degenerate_variance_tuning() -> CombatTuning {
    CombatTuning {
        view_range: ViewRange::new(TEST_VIEW_RANGE),
        melee: MeleeTuning {
            variance: FightVariance::new(0.0),
            ..MeleeTuning::default()
        },
        ..CombatTuning::default()
    }
}

fn run_degenerate_variance_strike() -> (Option<u16>, Option<u8>, Option<u8>, usize) {
    let mut app = battle_app_with_tuning(degenerate_variance_tuning());
    with_melee_log(&mut app);

    let situation = SituationBuilder::new()
        .with_gangers([
            strong_attacker(ground(5, 5), PLAYER, Direction::East),
            defenceless_target(ground(6, 5), ENEMY),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let (Some(attacker), Some(target)) = (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
    else {
        unreachable!("setup spawns one player attacker and one enemy target");
    };

    app.world_mut()
        .write_message(MeleeRequested::new(attacker, target));
    step(&mut app, 3);

    (
        hp_of(&app, target),
        wounds_of(&app, target),
        tu_of(&app, attacker),
        melee_hits(&app),
    )
}

#[test]
fn variance_zero_melee_strike_resolves_without_panic() {
    let mut app = battle_app_with_tuning(degenerate_variance_tuning());
    with_melee_log(&mut app);

    let situation = SituationBuilder::new()
        .with_gangers([
            strong_attacker(ground(5, 5), PLAYER, Direction::East),
            defenceless_target(ground(6, 5), ENEMY),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let (Some(attacker), Some(target)) = (ganger_of(&mut app, PLAYER), ganger_of(&mut app, ENEMY))
    else {
        unreachable!("setup spawns one player attacker and one enemy target");
    };
    let Some(hp_before) = hp_of(&app, target) else {
        unreachable!("the target carries an Hp pool");
    };

    app.world_mut()
        .write_message(MeleeRequested::new(attacker, target));
    step(&mut app, 3);

    assert!(
        melee_hits(&app) >= 1,
        "a variance-0.0 strike must RESOLVE and connect (MeleeResolved emitted)",
    );
    let Some(hp_after) = hp_of(&app, target) else {
        unreachable!("the target persists");
    };
    assert!(
        hp_after < hp_before,
        "the variance-0.0 connecting strike applies damage ({hp_after} < {hp_before})",
    );
}

#[test]
fn degenerate_variance_battle_replays_identically_under_same_seed() {
    let first = run_degenerate_variance_strike();
    let second = run_degenerate_variance_strike();
    assert_eq!(
        first, second,
        "the same seed + the same degenerate-variance battle must replay to \
         identical outcomes (hp, wounds, tu, hits)",
    );
}

#[test]
fn degenerate_exchange_leaves_subsequent_fight_draws_aligned() {
    let attacker = Fight::new(5.0);
    let defender = Fight::new(4.0);
    let live = FightVariance::new(0.25);

    let mut through_degenerate = FightRng::from_root(BattleSeed::new(SEED));
    let mut reference = FightRng::from_root(BattleSeed::new(SEED));

    let _ = opposed_fight(
        attacker,
        defender,
        FightVariance::new(0.0),
        &mut through_degenerate,
    );
    let _ = opposed_fight(attacker, defender, live, &mut reference);

    for i in 0..8 {
        let after_degenerate = opposed_fight(attacker, defender, live, &mut through_degenerate);
        let after_live = opposed_fight(attacker, defender, live, &mut reference);
        assert_eq!(
            after_degenerate, after_live,
            "exchange {i} after a degenerate opposed roll must equal the same-seed \
             exchange after a live one — the degenerate roll must consume exactly two draws",
        );
    }
}
