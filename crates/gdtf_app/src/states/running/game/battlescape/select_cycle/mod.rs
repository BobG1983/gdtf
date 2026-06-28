//! The battlescape Prev/Next selection-cycle cluster (GTW-458): the far-RIGHT vertical pair of
//! buttons on the bottom bar that cycle the [`SelectedShooter`](gdtf_battle_input::SelectedShooter)
//! through the PLAYER gang in deterministic `(z, y, x)` order, wrapping.
//!
//! A press on **Next** / **Prev** pushes [`ActIntent::SelectNext`](gdtf_battle_input::ActIntent::SelectNext)
//! / [`ActIntent::SelectPrev`](gdtf_battle_input::ActIntent::SelectPrev) onto the shared
//! act-intent seam — the SAME intents the `Tab` / `Shift+Tab` keys push (ADR-0001: keys +
//! buttons share ONE dispatch). The cluster sits INSIDE the bottom bar's padding and does NOT
//! change the bar's measured height.
//!
//! UI/view only: it reads nothing of the sim's combat rules; its ONLY write is the input
//! crate's act-intent seam. The cluster is static (spawned once on `BattleRunning`, never
//! respawned on update — [[ui-mutate-not-respawn]]).

mod components;
mod plugin;
mod systems;

pub(in crate::states::running::game::battlescape) use plugin::GameBattleScapeSelectCycleScenePlugin;

// Test-support-only re-export of the cluster ROOT + the Next / Prev button markers, carried
// toward `crate::test_support` so the AC tests can name them and assert the cluster's presence
// + width; gated so the binary build stays `unused`/`unreachable_pub`-clean (the weapon-panel
// marker re-export precedent).
#[cfg(feature = "test-support")]
crate::support_use! {
    components::{SelectCycleRoot, SelectNextButton, SelectPrevButton};
}
