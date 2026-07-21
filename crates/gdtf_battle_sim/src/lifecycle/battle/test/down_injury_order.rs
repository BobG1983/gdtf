//! GTW-728 — a downing hit that ALSO injures records the INJURY before the DOWN.
//!
//! The reported bug (live play 2026-07-14) was a ganger receiving an injury AFTER it was
//! downed by the SAME shot. The sim was never wrong: [`apply_hit`](crate::damage_resolution::apply_hit)
//! spends Wounds by severity tier and the terminal gate sets `Downed`/`Dead` in one tick,
//! and the named injury is rolled in-fold + applied the same tick
//! ([`apply_injury`](crate::acts::apply_injury) `.after(dispatch_fire)` in
//! [`SimSystems::Simulate`](crate::occupancy_sync::SimSystems::Simulate)). Canon PERMITS a
//! post-down injury from the same shot (`docs/combat/wounds-and-roster.md`,
//! `docs/combat/resolution.md`). The reversal was a pre-GTW-727 presentation artifact: the
//! playback cursor now paces every channel through the fixed-order act log, where the
//! consequence family (the injury) is recorded BEFORE the life family (the `Downed`
//! transition) — `record_acts`'s fixed family order (`act_log/record/pass.rs`): turn →
//! posture → movement → fire → consequence → **life LAST**.
//!
//! This drives ONE fire through the REAL `setup_battle_on_request` → `BattleSimPlugin`
//! `Simulate`/`Record` path (no stubs), on a target forced to one HP so the round both
//! DOWNS it and — with a penetrating weapon + paper armor + a catalog that draws a known
//! injury for any tabled `(part, severity)` — DRAWS a named injury, then reads
//! [`ActLog`](crate::act_log::ActLog) to pin the injury's entry BEFORE the down's. It
//! passes at HEAD (GTW-727's fixed family order) and would fail if the `Downed` transition
//! were emitted ahead of the same hit's injury, or the injury were not applied on a
//! downing hit.

use bevy::prelude::Entity;

use super::{
    injury::{
        aim_debuff_catalog, duel_entities, duel_situation, paper_armor_registry,
        penetrating_weapon_registry,
    },
    support::*,
};
use crate::{
    act_log::{ActDeed, ActLog, ActSeq},
    ganger::{Hp, Position},
    injuries::InflictedInjuries,
};

/// The outcome of one driven fire against a one-HP target: whether the round downed it,
/// whether its ledger gained a named injury, and — from the act log — the APPEND POSITIONS
/// of the target's [`ActDeed::Injured`] and [`ActDeed::LifeChanged`]`{ to: Downed }`
/// entries this run (the `since`-walk index is the append, hence sequence, order).
struct DownInjuryRun {
    /// The target ended the tick at [`LifeState::Downed`] (Hp depleted to 0, Wounds > 0).
    downed:            bool,
    /// The target's [`InflictedInjuries`] ledger gained at least one injury this round.
    ledger_has_injury: bool,
    /// The append position of the target's `Injured` act-log entry, if any.
    injured_index:     Option<usize>,
    /// The append position of the target's `LifeChanged { to: Downed }` entry, if any.
    downed_index:      Option<usize>,
}

/// Drive ONE battle at `seed`: install the penetrating weapon + paper armor + injury
/// catalog (so a landed round is a tabled, non-graze wound that draws a known injury),
/// force the target to one HP, fire the shooter at it point-blank, then advance a single
/// tick so `Simulate` resolves the shot and the `Record` pass (`.after(Simulate)`) appends
/// the tick's deeds.
fn run_down_injury(seed: u64) -> DownInjuryRun {
    let mut app = headless_app();
    // Override the harness's stock weapon/armor (its `fatal_bias` skews wounds Fatal, which
    // is NOT tabled) with a penetrating, zero-Fatal-bias weapon + zero-protection armor so a
    // landed round is a tabled Minor/Major/Critical wound, and a catalog that draws a known
    // injury for ANY tabled `(part, severity)` — the `injury.rs` recipe. A later insert wins,
    // so this must precede the setup that resolves each ganger's loadout.
    app.insert_resource(penetrating_weapon_registry());
    app.insert_resource(paper_armor_registry());
    let (registry, tables) = aim_debuff_catalog();
    app.insert_resource(registry);
    app.insert_resource(tables);
    app.world_mut().write_message(SetupBattleRequested::new(
        duel_situation(),
        BattleSeed::new(seed),
    ));
    // Settle the deferred bsn! ganger scenes + their weapon/armor relationships, and let the
    // Record pass seed each ganger's prior life state at Alive (so the fire tick records a
    // genuine Alive -> Downed transition rather than a silent first observation).
    for _ in 0..4 {
        app.update();
    }

    let (shooter, target) = duel_entities(&mut app);
    // Force the target to ONE HP so this single round depletes its Hp to 0 (the Downed
    // terminal gate), while its Wounds pool survives a non-fatal spend (so the gate is
    // Downed, not Dead). The bevy-traps #7 headless-test carve-out — a world_mut() write in
    // the test body, the same way the harness's `set_tu` drives a ganger's pool. Severity is
    // rolled from pen/toughness/part, not Hp, so this changes only WHICH terminal gate fires.
    if let Some(mut hp) = app.world_mut().get_mut::<Hp>(target) {
        *hp = Hp::new(1);
    }
    let target_cell = app
        .world()
        .get::<Position>(target)
        .map_or(Cell::new(10, 5), |p| p.cell());

    let mode = FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.2),
        ModeShots::new(1),
    );
    app.world_mut().write_message(FireRequested::new(
        shooter,
        mode,
        target_cell,
        Level::new(0),
    ));
    // One update: Simulate resolves the shot (apply_hit downs, dispatch_fire emits the
    // injury, apply_injury folds it into the ledger THIS tick) then the Record pass appends
    // the tick's deeds — Injured (consequence family) before LifeChanged (life family, last).
    app.update();

    let downed = matches!(
        app.world().get::<LifeState>(target),
        Some(LifeState::Downed)
    );
    let ledger_has_injury = app
        .world()
        .get::<InflictedInjuries>(target)
        .is_some_and(|ledger| !ledger.gained().is_empty());
    let (injured_index, downed_index) = act_log_indices(&app, target);
    DownInjuryRun {
        downed,
        ledger_has_injury,
        injured_index,
        downed_index,
    }
}

/// The append positions of `target`'s [`ActDeed::Injured`] and
/// [`ActDeed::LifeChanged`]`{ to: Downed }` act-log entries (the first of each), or `None`
/// when absent. Walking [`ActLog::since`](crate::act_log::ActLog::since) from the start
/// yields entries in append (sequence) order, so the enumerate index IS the recorded order.
fn act_log_indices(app: &App, target: Entity) -> (Option<usize>, Option<usize>) {
    let Some(log) = app.world().get_resource::<ActLog>() else {
        return (None, None);
    };
    let mut injured = None;
    let mut downed = None;
    for (index, entry) in log.since(ActSeq::START).enumerate() {
        if entry.actor() != target {
            continue;
        }
        match *entry.deed() {
            ActDeed::Injured { .. } => injured = injured.or(Some(index)),
            ActDeed::LifeChanged {
                to: LifeState::Downed,
                ..
            } => downed = downed.or(Some(index)),
            _ => {}
        }
    }
    (injured, downed)
}

/// THE GTW-728 REGRESSION: on the one tick a shot both DOWNS a ganger and INJURES it, the
/// act log records the injury BEFORE the down — so a cursor replaying the log never shows
/// the body fall before the wound that felled it.
///
/// Asserts both halves the ticket names:
/// - SAME-TICK correctness — the target ends [`LifeState::Downed`] AND its
///   [`InflictedInjuries`] ledger gained the injury (both landed in the one fire tick).
/// - ORDERING — the target's `Injured` act-log entry precedes its
///   `LifeChanged { to: Downed }` entry.
///
/// Passes at HEAD (GTW-727's fixed family order records consequence before life); fails if
/// the `Downed` transition were emitted ahead of the same hit's injury, or the injury were
/// not applied on a downing hit.
#[test]
fn a_downing_hit_records_its_injury_before_the_down() {
    // Sweep seeds for one whose single point-blank round lands a tabled (non-graze,
    // non-fatal) wound — which both downs the one-HP target AND draws a named injury.
    // Severity is RNG-driven, so not every seed qualifies (a graze injures nothing; a Fatal
    // kills rather than downs); the catalog covers every part/severity, so any tabled wound
    // rolls the injury. The seed set is the `injury.rs` sweep plus a few more for headroom.
    let mut found = None;
    for seed in [
        0xA1u64, 0xB2, 0xC3, 0xD4, 0xE5, 0xF6, 0x17, 0x28, 0x39, 0x4A, 0x5B, 0x6C, 0x7D, 0x8E,
    ] {
        let run = run_down_injury(seed);
        if run.downed && run.ledger_has_injury {
            found = Some(run);
            break;
        }
    }
    let Some(run) = found else {
        unreachable!(
            "across the seed sweep, at least one single round must BOTH down the one-HP target \
             and draw a named injury — the same-shot down+injury the regression is about"
        );
    };

    // SAME-TICK correctness: the one round both downed the target and gave it the injury.
    assert!(
        run.downed,
        "the round depleted the target's Hp to 0 -> LifeState::Downed",
    );
    assert!(
        run.ledger_has_injury,
        "the SAME round drew a named injury into the target's InflictedInjuries ledger",
    );

    // ORDERING (the GTW-728 regression): the injury is logged before the down.
    let Some(injured_index) = run.injured_index else {
        unreachable!("a downed+injured run must have recorded an Injured deed for the target");
    };
    let Some(downed_index) = run.downed_index else {
        unreachable!(
            "a downed+injured run must have recorded a LifeChanged -> Downed for the target"
        );
    };
    assert!(
        injured_index < downed_index,
        "the injury (append index {injured_index}) must be logged BEFORE the down (append \
         index {downed_index}) — record_acts records consequence(injury) before life(Downed); \
         a reversal is the GTW-728 defect",
    );
}
