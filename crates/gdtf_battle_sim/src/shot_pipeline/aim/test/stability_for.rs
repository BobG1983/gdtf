use crate::{
    aim::{stability_for, test::support::*},
    cover::HeightBand,
    faced_cell::faced_cell,
    ganger::{Direction, StanceKind},
    metric::CellLevel,
    stability::{StabilityTerms, SuppressionStability, stability},
    tuning::CombatTuning,
};

#[test]
fn stability_for_bit_equals_a_direct_stability_call() {
    let tuning = CombatTuning::default();
    let state = ShooterState::new(20, 20, 2, StanceKind::Standing, false, Direction::East);
    let shooter = state.as_shooter();
    let terms = StabilityTerms::default();

    let entry = cover_entry(HeightBand::High);
    let ledger = ledger_with_faced_cover(&shooter, entry);

    let (via_composer_cone, via_composer_recoil) = stability_for(&shooter, terms, &ledger, &tuning);

    let (cell, level) = faced_cell(shooter.position, shooter.facing);
    let faced = ledger.peek(&CellLevel::new(cell, level));
    let (direct_cone, direct_recoil) = stability(
        terms,
        *shooter.stance,
        faced,
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

#[test]
fn brace_at_faced_cell_is_steadier_than_an_empty_cell() {
    use crate::cover::CoverLedger;

    let tuning = CombatTuning::default();
    let state = ShooterState::new(15, 15, 1, StanceKind::Standing, false, Direction::South);
    let shooter = state.as_shooter();

    let braced_ledger = ledger_with_faced_cover(&shooter, cover_entry(HeightBand::High));
    let (braced_cone, _) =
        stability_for(&shooter, StabilityTerms::default(), &braced_ledger, &tuning);

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
