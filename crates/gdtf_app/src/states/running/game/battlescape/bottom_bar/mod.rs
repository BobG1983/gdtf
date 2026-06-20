//! The battlescape BOTTOM BAR (GTW-275 layout overhaul, item 6): the ONE opaque full-width
//! strip at the bottom of the screen — the only UI that reduces the world map.
//!
//! The map fills the window as the background; the status / hover panels are corner OVERLAYS
//! that contribute nothing to the viewport inset (items 2 / 3). The map ends at THIS bar's top
//! edge: `set_world_viewport` MEASURES the bar's [`BottomBarRoot`] `ComputedNode` height and
//! insets the world-camera viewport's BOTTOM by it (full width, no side / top inset — items 1 /
//! 4). The weapon panel sits INSIDE the bar (item 6, via its own plugin sizing itself to the
//! shared [`BOTTOM_BAR_H_VH`] height); the controls + contextual panels (GTW-277 / GTW-294)
//! drop in later. View-only.

mod components;
mod plugin;
mod systems;

pub(in crate::states::running::game::battlescape) use components::{
    BOTTOM_BAR_H_VH, BOTTOM_BAR_PAD_Y_VH,
};
pub(in crate::states::running::game::battlescape) use plugin::GameBattleScapeBottomBarScenePlugin;
// Re-exported so the sibling weapon-panel plugin can order `spawn_weapon_panel`
// `.after(spawn_bottom_bar)` — the bottom-bar root must exist before the weapon panel parents the
// Stance Panel under it (D4, the 2026-06-18 screenshot review).
pub(in crate::states::running::game::battlescape) use systems::spawn_bottom_bar;

// The bottom-bar ROOT marker — `pub` under `test-support` (the AC tests assert + measure it),
// `pub(crate)` otherwise (reachable by the sibling `set_world_viewport` system that measures its
// height for the world-map BOTTOM inset, `unreachable_pub`-clean in the binary). The
// `WeaponPanelRoot` precedent: an UNGATED `support_use!` serves BOTH the viewport AND the tests
// with no double-import, and widens in lockstep with the `support_item!` declaration.
crate::support_use!(components::BottomBarRoot;);
