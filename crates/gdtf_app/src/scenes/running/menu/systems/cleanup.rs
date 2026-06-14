//! `OnExit(RunningState::Menu)` cleanup of the menu's navigation edges (GTW-121).

use bevy::{input_focus::directional_navigation::DirectionalNavigationMap, prelude::*};

/// Clears the menu's entries from the global [`DirectionalNavigationMap`] on menu
/// exit.
///
/// The menu's button entities are despawned by
/// [`DespawnOnExit(RunningState::Menu)`](bevy::prelude::DespawnOnExit), but the
/// [`DirectionalNavigationMap`] is a **global** resource (not state-scoped), so
/// the edges keyed by those now-dead entity ids would linger as stale neighbors.
/// The menu is the **sole** writer of this map today, so clearing it wholesale on
/// exit is correct and ordering-independent — unlike a per-entity `remove`, it
/// does not need the (about-to-be-despawned) entities to still be alive when this
/// runs (the state-scoped despawn shares the same exit transition, so their
/// relative order is not guaranteed; clearing sidesteps that entirely).
pub(in crate::scenes::running::menu) fn clear_nav_map(
    mut nav_map: ResMut<DirectionalNavigationMap>,
) {
    nav_map.clear();
}
