//! `OnExit(AppState::Load)`: drop the Load-scoped resources.

use bevy::prelude::*;

use crate::scenes::load::resources::{LoadFailed, LoadHandles};

/// Removes the Load-scoped resources on exit from [`AppState::Load`](crate::states::AppState::Load).
///
/// Drops [`LoadHandles`] and [`LoadFailed`] (per bevy-traps rule 1, a non-built-in
/// state-scoped resource is removed in the `OnExit` system). It deliberately does
/// **not** remove the resolved [`GdtfTheme`](gdtf_ui::theme::GdtfTheme) **nor**
/// the [`ActiveThemeHandle`](gdtf_ui::theme::ActiveThemeHandle): both are the
/// exception that *persists* past `Load` so every later scene can read the theme
/// and the GTW-137 retheme system can keep filtering asset events against the live
/// handle (which also keeps the theme asset loaded for GTW-138's watcher).
/// `remove_resource` is a no-op when the resource is absent, so removing
/// [`LoadFailed`] (only present on the failure path) is always safe.
pub(in crate::scenes::load) fn cleanup(mut commands: Commands) {
    commands.remove_resource::<LoadHandles>();
    commands.remove_resource::<LoadFailed>();
}
