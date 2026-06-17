//! The battlescape status HUD panel (GTW-252): a themed `gdtf_ui` panel that shows
//! the selected player ganger's vitals (identity / stance / TU / HP+Wounds / life
//! state). The first discovery-driven cut of the real battle HUD — a real working
//! panel built from existing `gdtf_ui` primitives + the landed selection seam, to be
//! iterated. UI/view only: it reads the sim's vital components + the input crate's
//! `SelectedShooter`, owns no combat rule, and changes no sim/input.

mod components;
mod plugin;
mod systems;

pub(in crate::scenes::running::game::battlescape) use plugin::GameBattleScapeStatusPanelScenePlugin;

// Test-support-only re-export of the per-line text markers: widened to `pub` under
// `test-support` so the external integration tests can name them through
// `crate::test_support` to assert each line's `Text` content (AC2 / AC3), and gated so
// the production binary build stays `unused`/`unreachable_pub`-clean (the action-bar
// per-act-marker re-export precedent). The `StatusPanelRoot` is internal-only and is
// NOT re-exported.
#[cfg(feature = "test-support")]
crate::support_use! {
    components::{HpText, IdentityText, LifeText, StanceText, TuText, WeaponNameText};
}
