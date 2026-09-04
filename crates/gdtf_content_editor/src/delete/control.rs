//! The button a mode panel draws for the record its form holds.

use bevy_egui::egui;
use gdtf_assets::ContentMemberKey;

use super::{offered::entries_offered_on, registry::DeleteRegistry, request::DeleteRequest};
use crate::mode::{EditorMode, InjurySubTab};

/// The text on every delete button the editor draws.
pub(crate) const DELETE_BUTTON_LABEL: &str = "Delete record";

/// Draw the delete the open screen offers, answering the request a press asks for.
///
/// A screen with no entry, or a form holding no key, paints nothing.
pub(crate) fn delete_control(
    ui: &mut egui::Ui,
    registry: &DeleteRegistry,
    screen: (EditorMode, Option<InjurySubTab>),
    key: Option<&ContentMemberKey>,
) -> Option<DeleteRequest> {
    let (mode, sub_tab) = screen;
    let key = key?;
    let offered = entries_offered_on(registry, mode, sub_tab);
    let entry = offered.first()?;
    ui.button(DELETE_BUTTON_LABEL)
        .clicked()
        .then(|| DeleteRequest::new(entry.family().clone(), key.clone()))
}
