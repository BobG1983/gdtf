//! The editor model's state-scoped lifecycle: insert `OnEnter(DebugGangEditor)`, remove
//! `OnExit(DebugGangEditor)` (GTW-420, AC2 + C1).
//!
//! There is no built-in state-scoped RESOURCE in Bevy (`bevy-traps.md` #1), so the
//! [`EditableGang`] model is inserted in an `OnEnter` system and removed in an `OnExit` system,
//! and every reader guards on its presence. The insert system is ordered BEFORE
//! [`spawn_editor_screen`](super::spawn::spawn_editor_screen) so the screen sees the model.

use bevy::prelude::*;
use gdtf_battle_sim::GangRegistry;

use crate::states::running::gang_editor::model::EditableGang;

/// Inserts the [`EditableGang`] model on `OnEnter(RunningState::DebugGangEditor)` (AC2).
///
/// Loads the first gang from the [`GangRegistry`] if one is present (an editable copy), else
/// starts EMPTY ([`EditableGang::default`] via
/// [`from_registry`](EditableGang::from_registry)'s empty branch). The registry is read as
/// `Option<Res<…>>` because in a headless / asset-less harness it may be absent; absent ⇒ an
/// empty model, the same as a present-but-empty registry.
pub(in crate::states::running::gang_editor) fn insert_editable_gang(
    mut commands: Commands,
    registry: Option<Res<GangRegistry>>,
) {
    let model = registry
        .as_ref()
        .map_or_else(EditableGang::default, |registry| {
            EditableGang::from_registry(registry)
        });
    commands.insert_resource(model);
}

/// Removes the [`EditableGang`] model on `OnExit(RunningState::DebugGangEditor)` (C1 — the
/// OnEnter-insert paired with an OnExit-remove). The screen entities are torn down by their own
/// `DespawnOnExit` markers.
pub(in crate::states::running::gang_editor) fn remove_editable_gang(mut commands: Commands) {
    commands.remove_resource::<EditableGang>();
}
