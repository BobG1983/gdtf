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

/// Test-support re-exports for this panel (GTW-569 one-hop ledger): the cluster ROOT +
/// the Next / Prev button markers (GTW-458) the AC tests name through
/// `crate::test_support` to assert the cluster's presence + width. The crate-root ledger
/// (`src/test_support.rs`) re-exports these by explicit name directly from here — no
/// intermediate `mod.rs` climb. `pub(crate)` on the module (not `pub`) because the parent
/// chain is `pub(crate)`, so a `pub mod` here trips the workspace `unreachable_pub = deny`.
#[cfg(feature = "test-support")]
pub(crate) mod test_support {
    pub use super::components::{SelectCycleRoot, SelectNextButton, SelectPrevButton};
}
