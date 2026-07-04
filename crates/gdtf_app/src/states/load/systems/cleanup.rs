//! `OnExit(AppState::Load)`: drop the Load-scoped resources.

use bevy::prelude::*;

use crate::states::load::resources::{LoadFailed, LoadHandles};

/// Removes the Load-scoped resources on exit from [`AppState::Load`](crate::states::AppState::Load).
///
/// Drops [`LoadHandles`] and [`LoadFailed`] (per bevy-traps rule 1, a non-built-in
/// state-scoped resource is removed in the `OnExit` system). It deliberately does
/// **not** remove the resolved [`GdtfTheme`](gdtf_ui::theme::GdtfTheme), the
/// theme's persistent [`HotRonHandle`](gdtf_assets::HotRonHandle), **nor** the GTW-205
/// [`LoadedSituation`](crate::states::load::LoadedSituation): all three are the
/// exception that *persists* past `Load` — the theme so every later scene can read
/// it and the GTW-137 retheme system can keep filtering asset events against the
/// live handle (which also keeps the theme asset loaded for GTW-138's watcher), and
/// the [`LoadedSituation`](crate::states::load::LoadedSituation) so the authored
/// battlefield outlives `Load` for the Generation consumer (E10.5).
/// `remove_resource` is a no-op when the resource is absent, so removing
/// [`LoadFailed`] (only present on the failure path) is always safe.
///
/// SCAFFOLD DIVERGENCE (GTW-575): this cleanup stays BESPOKE rather than
/// collapsing into `scaffold::remove_scoped_resource` calls because its whole
/// point is the curated SPLIT above — which Load products are per-span
/// (removed here) versus deliberately persistent (never removed). Keeping the
/// removes and the persistence-exception rationale in ONE hand-written system
/// keeps that split auditable in one place (P9: a scene that outgrows the
/// scaffold graduates to its own system).
pub(in crate::states::load) fn cleanup(mut commands: Commands) {
    commands.remove_resource::<LoadHandles>();
    commands.remove_resource::<LoadFailed>();
}
