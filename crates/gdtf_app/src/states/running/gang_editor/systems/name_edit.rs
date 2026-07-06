//! The gang-NAME edit system: a commit on the gang-name field updates the model name
//! (GTW-420, AC3).
//!
//! The gang-name text field is built with [`spawn_text_field`](gdtf_ui::spawn_text_field) and
//! carries the [`GangNameField`] marker. When it COMMITS (Enter / blur), the widget raises a
//! [`TextFieldCommitted`] message naming the field entity + its
//! [`CommittedTextValue`](gdtf_ui::CommittedTextValue). This system reads that message, confirms
//! the committing field is the gang-name field, and sets the
//! [`EditableGang`](super::super::model::EditableGang) name to a [`GangName`] built from the
//! committed text.

use bevy::prelude::*;
use gdtf_battle_sim::GangName;
use gdtf_ui::TextFieldCommitted;

use crate::states::running::gang_editor::{components::GangNameField, model::EditableGang};

/// Updates the model [`GangName`] from a commit on the gang-name field (AC3).
///
/// Reads [`TextFieldCommitted`] with a [`MessageReader`] (buffered events are messages —
/// `bevy-traps.md` #4) and filters to the field carrying [`GangNameField`] via
/// [`Query::contains`], so a commit on any OTHER field is ignored. Guarded by the model's
/// presence via `Option<ResMut<…>>` (the model is a state-scoped resource — `bevy-traps.md`
/// #1) and registered `run_if(in_state(RunningState::DebugGangEditor))`.
pub(in crate::states::running::gang_editor) fn commit_gang_name(
    mut commits: MessageReader<TextFieldCommitted>,
    name_fields: Query<(), With<GangNameField>>,
    model: Option<ResMut<EditableGang>>,
) {
    let Some(mut model) = model else {
        return;
    };
    for commit in commits.read() {
        if name_fields.contains(commit.field()) {
            model.set_name(GangName::new(commit.value().value().to_owned()));
        }
    }
}
