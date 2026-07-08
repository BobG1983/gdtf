//! GTW-657: an authored ALL-ZERO `body_part_weights` table is INVALID DATA,
//! rejected at Load through the tuning artifact's native loud channel — the
//! `RonAsset<CombatTuning>` deserialize fails (the serde `try_from` validation in
//! `gdtf_battle_sim::tuning`), the load reaches `Failed`, and the hot-RON resolve
//! warns naming `core_tuning/combat.tuning.ron` and inserts the const-fallback
//! `CombatTuning::default()` — so the degenerate table can never reach the sim's
//! §4 weighted hit-location roll (whose zero-draw Torso fallback stays as
//! defense-in-depth only). The VALID sibling artifact (the fixture's copy of the
//! shipped `ui_theme.tuning.ron`) still resolves, and `Load` still transitions —
//! the rejection is loud, never fatal, never stranding (the `load_tuning.rs` AC6
//! failure-path precedent, sharpened to the all-zero-weights cause).
//!
//! Pre-GTW-657 this test was RED: the all-zero table parsed cleanly and REACHED
//! the sim as the live `CombatTuning` resource — the observed defect (an authored
//! table that would diverge seeded streams through the roll's zero-draw fallback,
//! the GTW-644 class).

use std::path::PathBuf;

use gdtf_app::test_support::{AppState, app_state, load_released};
use gdtf_battle_sim::tuning::CombatTuning;
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until, advance_until_resource_exists};
use gdtf_ui::theme::GdtfTheme;

/// Generous SAFETY-NET cap for the real-asset `advance_until` waits gated on an
/// async asset load resolving — a guard against a genuine never-resolve hang,
/// NOT a timing budget (the GTW-305 signal-poll convention, as in `load_tuning.rs`).
const LOAD_SAFETY_NET: u32 = 10_000;

/// Absolute path to the all-zero-weights fixtures root
/// (`tests/fixtures/all_zero_weights_root`), whose `core_tuning/combat.tuning.ron`
/// is a copy of the shipped tuning EXCEPT its `body_part_weights` table is all
/// zero — so ONLY the GTW-657 weight-table rejection fails the tuning branch,
/// while the sibling `core_tuning/ui_theme.tuning.ron` (and the symlinked
/// `fonts/`) stay VALID.
fn all_zero_weights_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("all_zero_weights_root")
}

/// GTW-657 — the all-zero `body_part_weights` table is rejected at Load: the
/// tuning deserialize fails, the resolve falls back to exactly
/// [`CombatTuning::default()`] (whose weights are the non-zero doc defaults), the
/// valid theme sibling still loads, and `Load` still transitions.
///
/// Pin-discriminating: pre-fix the fixture's tuning parsed fine, so the resolved
/// resource carried the ALL-ZERO table (≠ default) and the first assertion went
/// red — the "reaches the sim today" observation. Post-fix only the const
/// fallback can appear. A regression that re-admits the table at deserialize
/// turns this red again.
#[test]
fn all_zero_weights_tuning_is_rejected_and_falls_back_to_default() {
    let mut app = GdtfLoadTestAppBuilder::with_asset_root(all_zero_weights_root())
        .starting_in(AppState::Load)
        .build();

    // Signal-poll: the rejection path still produces a CombatTuning (the const
    // fallback), so the same inserted-resource signal fires — the rejection can
    // never strand Load. Cap is a safety net (GTW-305).
    advance_until_resource_exists::<CombatTuning>(&mut app, LOAD_SAFETY_NET);

    let tuning = app.world().get_resource::<CombatTuning>();
    assert_eq!(
        tuning,
        Some(&CombatTuning::default()),
        "an all-zero body_part_weights table must be REJECTED at load — the resolve \
         must insert exactly the const-fallback default tuning, never the authored \
         all-zero table",
    );

    // The VALID sibling artifact still loads: the theme branch resolves normally —
    // the rejection is scoped to the offending tuning file, not the load pass.
    let theme_loaded = advance_until(
        &mut app,
        |app| app.world().get_resource::<GdtfTheme>().is_some(),
        LOAD_SAFETY_NET,
    );
    assert!(
        theme_loaded,
        "the valid sibling core_tuning/ui_theme.tuning.ron must still resolve — the \
         all-zero-weights rejection must not take down the load pass",
    );

    // And Load still transitions — loud, never fatal, never stranding.
    // Intro is TRANSIENT — probe via `load_released`, never `== Intro` (GTW-589/GTW-601).
    let released = advance_until(&mut app, load_released, LOAD_SAFETY_NET);
    assert!(
        released,
        "even with the all-zero-weights tuning rejected, Load must release to Intro \
         (or beyond) with the default tuning; last AppState was {:?}",
        app_state(&app),
    );
}
