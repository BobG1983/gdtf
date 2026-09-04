//! The routes for the Theme tab, whose only field is its draft's name, and the Prefab tab,
//! which holds no draft at all.

use bevy::prelude::ResMut;

use crate::{
    mcp::{
        commands::write::{
            form_fault::FormWriteFault,
            set_field::command::{SetFieldRefusal, Written, no_field_of_its_own, present},
        },
        wire::{EditorDraftNameNet, EditorFieldNet, ThemeFieldNet},
    },
    theme_form::ThemeDraft,
};

/// Route a field write on the Theme tab, whose one arm is the display name its name box writes.
pub(in crate::mcp::commands::write::set_field) fn theme(
    draft: Option<&mut ResMut<'_, ThemeDraft>>,
    field: EditorFieldNet,
) -> Written {
    match field {
        EditorFieldNet::Theme(ThemeFieldNet::Name(name)) => {
            let draft = present(draft)?;
            draft.set_display_name((*name).clone());
            Ok(EditorFieldNet::Theme(ThemeFieldNet::Name(
                EditorDraftNameNet::new(draft.display_name()),
            )))
        }
        EditorFieldNet::Terrain(_)
        | EditorFieldNet::Armor(_)
        | EditorFieldNet::Sprite(_)
        | EditorFieldNet::Attachment(_)
        | EditorFieldNet::Injury(_)
        | EditorFieldNet::Weighting(_)
        | EditorFieldNet::MeleeWeapon(_)
        | EditorFieldNet::Gang(_)
        | EditorFieldNet::Weapon(_)
        | EditorFieldNet::Field(_) => no_field_of_its_own(draft),
    }
}

/// Route a field write on the Prefab tab, the map canvas, which holds no draft and no field.
pub(in crate::mcp::commands::write::set_field) fn prefab(field: EditorFieldNet) -> Written {
    match field {
        EditorFieldNet::Terrain(_)
        | EditorFieldNet::Armor(_)
        | EditorFieldNet::Sprite(_)
        | EditorFieldNet::Attachment(_)
        | EditorFieldNet::Injury(_)
        | EditorFieldNet::Weighting(_)
        | EditorFieldNet::MeleeWeapon(_)
        | EditorFieldNet::Gang(_)
        | EditorFieldNet::Weapon(_)
        | EditorFieldNet::Field(_)
        | EditorFieldNet::Theme(_) => Err(SetFieldRefusal::Fault(FormWriteFault::ForeignArm)),
    }
}
