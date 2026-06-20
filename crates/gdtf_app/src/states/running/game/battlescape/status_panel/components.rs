//! Marker component for the battlescape status HUD panel (GTW-278).
//!
//! The status panel is a battle-scoped `gdtf_ui` panel showing the selected player
//! ganger's stat block (portrait / name / faction / stance / TU+HP bars / Wounds pips /
//! wound-name list — the shared
//! [`stat_block`](super::super::stat_block)). Its tree is a
//! [`spawn_panel`](gdtf_ui::spawn_panel) box (the [`StatusPanelRoot`] marker) holding the
//! one shared stat-block subtree; the per-widget markers and the per-update mutate live in
//! the shared stat-block module, so the panel needs only its own root marker.

use bevy::prelude::*;

/// Marks the **root** node of the status-panel tree (the
/// [`spawn_panel`](gdtf_ui::spawn_panel) box holding the shared stat block), so the
/// `OnExit(BattleRunning)` despawn finds and recursively tears down the whole panel by
/// this one marker rather than tracking each child.
///
/// The root is an ABSOLUTE, fixed-% (`Val::Vw`/`Val::Vh`) OVERLAY anchored to the window's
/// TOP-LEFT corner (GTW-275): it hovers OVER the full-window map and contributes NOTHING to
/// the world-map viewport inset (only the bottom bar reduces the map). It stays
/// `pub(in …battlescape)`, like [`ActionBarRoot`](super::super::action_bar), so the
/// sibling battlescape-level layout code can still find the root by marker; it is NOT
/// widened through `support_item!`/the test-support chain — it stays inside the battlescape
/// neighborhood and `unreachable_pub`-clean. A unit marker: presence on an entity is the
/// whole signal (no-bare-types rule).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(in crate::states::running::game::battlescape) struct StatusPanelRoot;

/// Marks the status panel's shared stat-block root, so the update system finds THIS
/// panel's [`StatBlockRefs`](super::super::stat_block::StatBlockRefs) (the hover panel's
/// stat block carries the same handle but a different panel marker). A unit marker:
/// presence on an entity is the whole signal (no-bare-types rule).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(in crate::states::running::game::battlescape::status_panel) struct StatusStatBlock;
