//! The battlescape hover-inspect panel (GTW-274): the TWIN of the status panel, anchored
//! TOP-RIGHT (the GTW-271 right margin). It inspects whatever the cursor hovers — reading
//! the [`HoveredCell`](gdtf_battle_input::HoveredCell) + the
//! [`OccupancyGrid`](gdtf_battle_sim::OccupancyGrid):
//!
//! - a hovered GANGER → the shared [`stat_block`](super::stat_block), with its NAME line
//!   color tinted by the ganger's `Faction` (enemy red-ish, player the normal theme color);
//! - a hovered non-floor OBJECT (wall / cover) → an object block (hardness + integrity);
//! - bare floor / nothing → the panel is `Visibility::Hidden`.
//!
//! Pure VIEW; mutate-in-place on `HoveredCell` change ([[ui-mutate-not-respawn]]). It
//! mirrors the status-panel 4-file pattern (`mod` / `plugin` / `components` / `systems`)
//! and the same `BattleRunning` lifecycle.

mod components;
mod plugin;
mod systems;

pub(in crate::scenes::running::game::battlescape) use plugin::GameBattleScapeHoverPanelScenePlugin;

// The panel root marker — `pub` under `test-support` (the hover test asserts the bare-floor
// Hidden state), `pub(crate)` otherwise (reachable from the battlescape viewport system that
// measures its width for the RIGHT inset, `unreachable_pub`-clean in the binary).
crate::support_use!(components::HoverPanelRoot;);

// Test-support-only re-export of the hover-panel's host + object markers, widened to `pub`
// under `test-support` so the external integration test can name them through
// `crate::test_support`, and gated so the production binary build stays
// `unreachable_pub`-clean.
#[cfg(feature = "test-support")]
crate::support_use! {
    components::{HoverObjectBar, HoverObjectBlock, HoverObjectText, HoverStatBlockHost};
}
