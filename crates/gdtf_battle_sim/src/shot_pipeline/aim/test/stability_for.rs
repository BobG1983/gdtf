//! `stability_for` composer tests — it WRAPS the landed `stability` verb (AC1)
//! and the faced-cell brace gate makes a satisfying band steadier than an empty
//! cell (AC2).

use crate::{
    aim::{stability_for, test::support::*},
    cover::HeightBand,
    faced_cell::faced_cell,
    ganger::{Direction, StanceKind},
    metric::CellLevel,
    stability::{StabilityTerms, SuppressionStability, stability},
    tuning::CombatTuning,
};

/// AC1 — `stability_for` returns the SAME `(ConeMult, RecoilGrowth)` as a
/// direct [`stability`] call with the same `(terms, stance, faced cover,
/// tuning)`: bit-equality proving it WRAPS (does not re-derive) the landed verb.
/// A `CoverEntry` is inserted at the faced cell so the direct call's `faced`
/// argument is exactly what the composer peeks.
#[test]
fn stability_for_bit_equals_a_direct_stability_call() {
    let tuning = CombatTuning::default();
    let state = ShooterState::new(20, 20, 2, StanceKind::Standing, false, Direction::East);
    let shooter = state.as_shooter();
    // Every GTW-573 term at its zero identity — the un-braced / un-mounted baseline.
    let terms = StabilityTerms::default();

    // Cover at the faced cell so the composer's peek returns Some(entry).
    let entry = cover_entry(HeightBand::High);
    let ledger = ledger_with_faced_cover(&shooter, entry);

    let (via_composer_cone, via_composer_recoil) = stability_for(&shooter, terms, &ledger, &tuning);

    // The direct call with the EXACT same inputs the composer fed the verb.
    let (cell, level) = faced_cell(shooter.position, shooter.facing);
    let faced = ledger.peek(&CellLevel::new(cell, level));
    let (direct_cone, direct_recoil) = stability(
        terms,
        *shooter.stance,
        faced,
        // The shooter (via `ShooterState::new`) is un-suppressed, so the composer feeds the
        // identity suppression term — match it here for the bit-equality.
        SuppressionStability::none(),
        &tuning.cone_stability,
    );

    assert_eq!(
        (*via_composer_cone).to_bits(),
        (*direct_cone).to_bits(),
        "stability_for must bit-equal a direct stability call (cone_mult)",
    );
    assert_eq!(
        (*via_composer_recoil).to_bits(),
        (*direct_recoil).to_bits(),
        "stability_for must bit-equal a direct stability call (recoil_growth)",
    );
}

/// AC2 — a faced cell whose cover band satisfies the per-stance brace gate
/// yields a strictly STEADIER (smaller `ConeMult`) `stability_for` than the
/// same shooter facing an EMPTY cell — RELATION, never a pinned score. A
/// standing shooter's gate is HIGH (resolution.md §1a), so a HIGH faced wall
/// braces; an empty ledger (no faced cover) withholds the brace.
#[test]
fn brace_at_faced_cell_is_steadier_than_an_empty_cell() {
    use crate::cover::CoverLedger;

    let tuning = CombatTuning::default();
    let state = ShooterState::new(15, 15, 1, StanceKind::Standing, false, Direction::South);
    let shooter = state.as_shooter();

    // Braced: a HIGH wall at the faced cell satisfies the standing gate.
    let braced_ledger = ledger_with_faced_cover(&shooter, cover_entry(HeightBand::High));
    let (braced_cone, _) =
        stability_for(&shooter, StabilityTerms::default(), &braced_ledger, &tuning);

    // Unbraced: an empty ledger — no cover at the faced cell.
    let empty = CoverLedger::new();
    let (empty_cone, _) = stability_for(&shooter, StabilityTerms::default(), &empty, &tuning);

    assert!(
        *braced_cone < *empty_cone,
        "a satisfying faced band must brace (steadier, lower cone_mult) vs an empty cell: \
         braced {} vs empty {}",
        *braced_cone,
        *empty_cone,
    );
}
