//! The "Add member" system: a press appends a default member to the model AND spawns a row in
//! the member-list shell (GTW-420, AC4).
//!
//! Pressing the [`AddMemberButton`] appends [`EditableMember::default_member`] to the
//! [`EditableGang`](super::super::model::EditableGang) (the model effect) and spawns a minimal
//! [`MemberRow`](super::super::components::MemberRow) under the
//! [`MemberListHost`](super::super::components::MemberListHost) (the view effect). The two
//! effects keep the shell in lockstep with the model — one row per member.

use bevy::{prelude::*, ui::Interaction};
use gdtf_ui::theme::GdtfTheme;

use crate::states::running::editor::{
    components::{AddMemberButton, MemberListHost},
    model::{EditableGang, EditableMember},
    systems::spawn::spawn_member_row,
};

/// Appends a default member + spawns its row when "Add member" is pressed (AC4).
///
/// Reads the button's [`Interaction`] filtered `Changed<Interaction>` + `== Pressed` so it
/// fires once per press edge (the menu-action precedent), appends
/// [`EditableMember::default_member`] to the model, and parents a fresh minimal row under the
/// member-list host. Guarded on the model + theme presence (state-scoped resource —
/// `bevy-traps.md` #1) and registered `run_if(in_state(RunningState::DebugEditor))`.
pub(in crate::states::running::editor) fn add_member_on_press(
    mut commands: Commands,
    buttons: Query<&Interaction, (Changed<Interaction>, With<AddMemberButton>)>,
    hosts: Query<Entity, With<MemberListHost>>,
    theme: Option<Res<GdtfTheme>>,
    model: Option<ResMut<EditableGang>>,
) {
    let pressed = buttons
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed);
    if !pressed {
        return;
    }
    let (Some(mut model), Some(theme)) = (model, theme) else {
        return;
    };
    model.add_default_member();
    let row_name = EditableMember::default_member();
    let Some(host) = hosts.iter().next() else {
        return;
    };
    let row = spawn_member_row(&mut commands, &theme, row_name.name().as_str());
    commands.entity(host).add_child(row);
}
