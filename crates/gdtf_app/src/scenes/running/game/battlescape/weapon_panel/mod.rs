//! The battlescape weapon panel (GTW-275, bottom-left): a themed `gdtf_ui` panel showing
//! the selected player ganger's weapon — a graphic PLACEHOLDER (no per-weapon art / items
//! atlas), the weapon's [`WeaponName`](gdtf_battle_sim::WeaponName), the magazine
//! `"cur/max"` round count, a LIVE Reload button, and throwable placeholder slots. The
//! Reload button is the VIEW end of the real, TU-costed reload act (the sim's
//! `dispatch_reload`): a press pushes [`ActIntent::Reload`](gdtf_battle_input::ActIntent::Reload)
//! onto the shared act-intent seam, draining to a
//! [`ReloadRequested`](gdtf_battle_sim::acts::ReloadRequested) for the selection.
//!
//! UI/view only: it reads the sim's weapon components + the input crate's `SelectedShooter`,
//! owns no combat rule, and its ONLY write is the act-intent seam. Mutate-in-place on
//! selection change ([[ui-mutate-not-respawn]]).

mod components;
mod plugin;
mod systems;

pub(in crate::scenes::running::game::battlescape) use plugin::GameBattleScapeWeaponPanelScenePlugin;

// The panel ROOT marker (GTW-275 / AC9) — `pub` under `test-support` (the AC tests assert
// the panel's presence), `pub(crate)` otherwise (reachable by the battlescape
// `set_world_viewport` system that measures its size for the LEFT/BOTTOM inset,
// `unreachable_pub`-clean in the binary). The `HoverPanelRoot` precedent: an UNGATED
// `support_use!` serves BOTH the viewport AND the tests with no double-import.
crate::support_use!(components::WeaponPanelRoot;);

// Test-support-only re-export of the weapon-panel's content / name / magazine / reload
// markers (GTW-275), carried toward `crate::test_support` so the AC tests can name them;
// gated so the binary build stays `unused`/`unreachable_pub`-clean.
#[cfg(feature = "test-support")]
crate::support_use! {
    components::{ReloadButton, WeaponContent, WeaponMagazineText, WeaponNameText};
}
