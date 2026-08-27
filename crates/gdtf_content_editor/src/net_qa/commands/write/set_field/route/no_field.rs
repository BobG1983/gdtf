//! The routes for the two tabs that own no field arm of their own.

use bevy::prelude::ResMut;

use crate::{
    net_qa::{
        commands::write::{
            form_fault::FormWriteFault,
            set_field::command::{SetFieldRefusal, Written, no_field_of_its_own},
        },
        wire::EditorFieldNet,
    },
    theme_form::ThemeDraft,
};

/// Route a field write on the Theme tab, which owns no field arm of its own.
pub(in crate::net_qa::commands::write::set_field) fn theme(
    draft: Option<&mut ResMut<'_, ThemeDraft>>,
    field: EditorFieldNet,
) -> Written {
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
        | EditorFieldNet::Field(_) => no_field_of_its_own(draft),
    }
}

/// Route a field write on the Prefab tab, the map canvas, which holds no draft and no field.
pub(in crate::net_qa::commands::write::set_field) fn prefab(field: EditorFieldNet) -> Written {
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
        | EditorFieldNet::Field(_) => Err(SetFieldRefusal::Fault(FormWriteFault::ForeignArm)),
    }
}
