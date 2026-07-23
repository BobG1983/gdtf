//! Keyboard focus-navigation for the battlescape HUD (GTW-782).
//!
//! Wires `gdtf_ui`'s focus-navigation framework into the multi-panel battlescape HUD: a
//! typed-`Keybinds` → framework-message [`bridge`] that drives panel focus-nav / cancel
//! from Tab / Escape while a panel holds focus, the inter-panel [`topology`] rebuild that
//! keeps the Tab traversal chain current over the dynamically shown/hidden panel buttons,
//! and the [`outline`] focus highlight. The context primitives it gates on
//! ([`PanelNavOrder`](gdtf_battle_input::PanelNavOrder) /
//! [`focused_panel_button`](gdtf_battle_input::focused_panel_button)) live in
//! `gdtf_battle_input` so the existing Tab / Escape handlers there share the same gate.

mod bridge;
mod outline;
mod plugin;
#[cfg(test)]
mod test;
mod topology;

pub(in crate::states::running::game::battlescape) use plugin::GameBattleScapeFocusNavScenePlugin;
pub(in crate::states::running::game::battlescape) use topology::{
    ACTION_BAR_NAV_BASE, CONTEXTUAL_NAV_BASE, WEAPON_NAV_BASE,
};
