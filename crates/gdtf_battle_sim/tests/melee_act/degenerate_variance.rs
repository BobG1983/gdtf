//! GTW-640 + GTW-644 — DEGENERATE tunable ranges on the REAL melee path.
//!
//! `FightVariance` is documented-legal down to `0.0` and hot-reloadable, so the §7
//! per-side band `[1 − v, 1 + v]` can be width-zero (or inverted) at runtime. The
//! contract: a degenerate band must NEVER panic, must resolve the strike (both roll
//! factors collapse to the band's midpoint `1.0`), must replay identically under the
//! same seed, and must consume EXACTLY the two `FightRng` draws a live band does —
//! so one degenerate exchange can never shear the fight stream's subsequent draws.

use gdtf_battle_sim::{
    acts::MeleeRequested,
    ganger::{Direction, Fight},
    melee::opposed_fight,
    rng::{BattleSeed, FightRng},
    test_support::SituationBuilder,
    tuning::{CombatTuning, FightVariance, MeleeTuning, ViewRange},
};

use super::harness::*;

/// A `CombatTuning` with the §7 fight variance pinned DEGENERATE (`0.0`) — an explicit
/// test input (a legal tuning state, never a shipped magnitude).
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

/// Run the fixed variance-`0.0` battle once: strong player attacker at (5,5) facing the
/// defenceless enemy at (6,5), one `MeleeRequested` strike, settled — returning the
/// observable outcome tuple (target HP / target Wounds / attacker TU / resolved-hit count).
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

    // Drive the strike THROUGH the buffered MeleeRequested (the message the input seam
    // writes) — the REAL dispatch_melee → resolve_melee_strike → opposed_fight path.
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

// === GTW-640 C1a — variance 0.0 is PLAYABLE: the strike resolves without panic. ===

/// GTW-640 (C1a): a melee strike under `FightVariance` `0.0` RESOLVES — no panic, the
/// opposed roll's factors collapse to the midpoint `1.0`, the strong attacker beats the
/// zero-Fight defender (the §7 degenerate `def ≤ 0` connect), and the §5→§7→§6 chain
/// applies real damage. RED before the safe-draw fix: `opposed_fight` asked rand for the
/// EMPTY range `1.0..1.0` and panicked ("cannot sample empty range").
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

    // The strike RESOLVED (reaching here at all is the no-panic clause) and CONNECTED:
    // the FX signal emitted and the target's HP went down.
    assert!(
        melee_hits(&app) >= 1,
        "GTW-640: a variance-0.0 strike must RESOLVE and connect (MeleeResolved emitted)",
    );
    let Some(hp_after) = hp_of(&app, target) else {
        unreachable!("the target persists");
    };
    assert!(
        hp_after < hp_before,
        "GTW-640: the variance-0.0 connecting strike applies damage ({hp_after} < {hp_before})",
    );
}

// === GTW-640 + GTW-644 C4 — the determinism pin. ===

/// C4 (replay half): the SAME seeded resolution with the degenerate range configured,
/// run twice, produces IDENTICAL outcomes — HP, Wounds, TU, and resolved-hit count all
/// replay exactly (the seeded-RNG determinism pillar holds through the degenerate path).
#[test]
fn degenerate_variance_battle_replays_identically_under_same_seed() {
    let first = run_degenerate_variance_strike();
    let second = run_degenerate_variance_strike();
    assert_eq!(
        first, second,
        "GTW-640/644: the same seed + the same degenerate-variance battle must replay to \
         identical outcomes (hp, wounds, tu, hits)",
    );
}

/// C4 (stream-alignment half): a DEGENERATE opposed exchange consumes EXACTLY the
/// stream a live one does — two same-seeded [`FightRng`]s, one routed through a
/// degenerate (`v = 0.0`) first exchange and one through a live first exchange, stay
/// aligned: their subsequent NON-degenerate sibling draw sequences match value-for-value.
/// A draw-skipping guard (the GTW-644 defect shape) would shear every exchange after
/// the degenerate one.
#[test]
fn degenerate_exchange_leaves_subsequent_fight_draws_aligned() {
    let attacker = Fight::new(5.0);
    let defender = Fight::new(4.0);
    // An arbitrary non-degenerate width (a test input, not a shipped magnitude).
    let live = FightVariance::new(0.25);

    let mut through_degenerate = FightRng::from_root(BattleSeed::new(SEED));
    let mut reference = FightRng::from_root(BattleSeed::new(SEED));

    // First exchange: DEGENERATE on one stream, live on the other — both must advance
    // the cursor by exactly two draws.
    let _ = opposed_fight(
        attacker,
        defender,
        FightVariance::new(0.0),
        &mut through_degenerate,
    );
    let _ = opposed_fight(attacker, defender, live, &mut reference);

    // Sibling sequence: identical LIVE exchanges on both streams must now match
    // value-for-value — the degenerate exchange left the stream aligned.
    for i in 0..8 {
        let after_degenerate = opposed_fight(attacker, defender, live, &mut through_degenerate);
        let after_live = opposed_fight(attacker, defender, live, &mut reference);
        assert_eq!(
            after_degenerate, after_live,
            "GTW-644: exchange {i} after a degenerate opposed roll must equal the same-seed \
             exchange after a live one — the degenerate roll must consume exactly two draws",
        );
    }
}
