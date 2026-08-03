use super::super::*;

#[test]
fn visibility_leaves_resolve_to_documented_defaults() {
    let tuning = CombatTuning::default();
    assert_eq!(
        *tuning.view_range, 14u16,
        "CombatTuning::default view_range must be the documented 14 Chebyshev cells",
    );
    assert_eq!(
        (*tuning.explored_dim).to_bits(),
        0.55_f32.to_bits(),
        "CombatTuning::default explored_dim must be the documented 0.55 RGB modulate",
    );
}
