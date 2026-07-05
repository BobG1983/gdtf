//! GTW-338 — the documented-default contract on `CombatTuning::default()`.

use super::super::*;

/// GTW-338 — the two squad-FOV visibility leaves resolve to their **documented
/// defaults** on the const-fallback `CombatTuning::default()` path, so a headless
/// fixture with NO loaded asset still gets a real view range and explored dim
/// (the same fallback the failed-load and asset-less harnesses rely on).
///
/// Unlike the real-asset loader tests (value-agnostic by the brittle-test rule), this
/// pins the `Default` impl this ticket authored — the defaults ARE the design contract
/// (visibility.md §"Tunables": `view_range` 14, `explored_dim` 0.55). The check is
/// exact: view range is an integer count; the dim's `f32` is an exactly-representable
/// literal compared by bit pattern (no `float_cmp` lint, no epsilon).
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
