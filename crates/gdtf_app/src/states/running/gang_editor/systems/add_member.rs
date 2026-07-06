//! The "Add member" system: a press appends a default member to the model AND spawns a collapsed
//! row in the member-list scroll list (GTW-420 AC4 + GTW-425 collapsed row).
//!
//! Pressing the [`AddMemberButton`] appends [`EditableMember::default_member`] to the
//! [`EditableGang`](super::super::model::EditableGang) (the model effect) and spawns a collapsed
//! [`MemberRow`](super::super::components::MemberRow) — keyed to the new member's index, with its
//! pip / portrait / name field / weapon + armor dropdowns / delete button — into the member-list
//! scroll list's [`ScrollListArea`](gdtf_ui::ScrollListArea) (NOT the [`MemberListHost`] grid root
//! frame — the scroll-list parenting rule, GTW-421/422). The two effects keep the list in lockstep
//! with the model — one row per member.

use bevy::{prelude::*, ui::Interaction};
use gdtf_battle_sim::{ArmorRegistry, GangerStatTuning, WeaponRegistry};
use gdtf_ui::{ScrollListArea, theme::GdtfTheme};

use crate::states::running::gang_editor::{
    components::{AddMemberButton, MemberListHost},
    model::EditableGang,
    systems::spawn::{sorted_armor_options, sorted_weapon_options, spawn_member_row},
};

/// Appends a default member + spawns its collapsed row when "Add member" is pressed (AC4 / C1).
///
/// Reads the button's [`Interaction`] filtered `Changed<Interaction>` + `== Pressed` so it fires
/// once per press edge (the menu-action precedent), appends [`EditableMember::default_member`] to
/// the model (capturing the new member's index), and spawns a collapsed row keyed to that index
/// into the scroll-list AREA. The dropdown options come from the global [`WeaponRegistry`] /
/// [`ArmorRegistry`] (all loaded keys — C2); the new row's readonly derived-stat displays are
/// seeded from the GTW-384 [`GangerStatTuning`] derivation (C3). Guarded on the model + theme
/// presence (state-scoped resource — `bevy-traps.md` #1) and registered
/// `run_if(in_state(RunningState::DebugGangEditor))`.
/// The row is parented into the [`ScrollListArea`] via a deferred command (it queries the
/// [`MemberListHost`] frame for its area child, the GTW-422 palette precedent).
pub(in crate::states::running::gang_editor) fn add_member_on_press(
    mut commands: Commands,
    buttons: Query<&Interaction, (Changed<Interaction>, With<AddMemberButton>)>,
    theme: Option<Res<GdtfTheme>>,
    weapons: Option<Res<WeaponRegistry>>,
    armor: Option<Res<ArmorRegistry>>,
    tuning: Option<Res<GangerStatTuning>>,
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
    let index = model.add_default_member();
    let weapon_options = sorted_weapon_options(weapons.as_deref());
    let armor_options = sorted_armor_options(armor.as_deref());
    // The GTW-384 derivation weights the new row's readonly stat displays seed from; an absent
    // resource falls back to the const-default weights (C3).
    let tuning = tuning.as_deref().cloned().unwrap_or_default();
    let Some(member) = model.member_at(index) else {
        return;
    };
    let row = spawn_member_row(
        &mut commands,
        &theme,
        index,
        member,
        &weapon_options,
        &armor_options,
        &tuning,
    );
    // Parent the new row into the scroll list's AREA (the clipping viewport), found via the
    // MemberListHost frame's ScrollListArea child — NOT the frame itself (the parenting rule).
    commands.queue(move |world: &mut World| {
        let Some(area) = member_list_scroll_area(world) else {
            return;
        };
        if let Ok(mut area_entity) = world.get_entity_mut(area) {
            area_entity.add_child(row);
        }
    });
}

/// Find the member-list [`ScrollListArea`] — the clipping, scrolling viewport child of the
/// [`MemberListHost`] scroll-list grid root frame (the GTW-421/422 parenting rule).
///
/// The [`MemberListHost`] marker rides the scroll-list ROOT FRAME; the rows must hang on the area
/// among its children, not the frame itself. Returns [`None`] if the frame or its area is not yet
/// present.
fn member_list_scroll_area(world: &mut World) -> Option<Entity> {
    let frame = world
        .query_filtered::<Entity, With<MemberListHost>>()
        .iter(world)
        .next()?;
    let children = world.get::<Children>(frame)?;
    children.iter().find(|child| {
        world
            .get_entity(*child)
            .is_ok_and(|entity| entity.contains::<ScrollListArea>())
    })
}
