//! Route one Weapon field arm to the widget group that draws it.

use super::{dot, on_death, plain};
use crate::{
    net_qa::{
        commands::write::form_fault::FormWriteFault,
        wire::{EditorDraftNameNet, EditorFieldNet},
    },
    weapon_form::WeaponDraft,
};

/// Write one Weapon field, answering the field as the draft stores it.
pub(in crate::net_qa::commands::write::set_field) fn write(
    draft: &mut WeaponDraft,
    field: EditorFieldNet,
) -> Result<EditorFieldNet, FormWriteFault> {
    if let EditorFieldNet::WeaponName(name) = field {
        draft.set_name((*name).clone());
        return Ok(EditorFieldNet::WeaponName(EditorDraftNameNet::new(
            draft.name(),
        )));
    }
    let spec = draft.spec_mut();
    match field {
        EditorFieldNet::WeaponDot(_)
        | EditorFieldNet::WeaponDotDamage(_)
        | EditorFieldNet::WeaponDotTurns(_)
        | EditorFieldNet::WeaponDotDamageType(_) => dot::write(spec, field),
        EditorFieldNet::WeaponOnDeath(_)
        | EditorFieldNet::WeaponOnDeathVariant(_)
        | EditorFieldNet::WeaponOnDeathHitType(_)
        | EditorFieldNet::WeaponOnDeathDamage(_)
        | EditorFieldNet::WeaponOnDeathDamageType(_)
        | EditorFieldNet::WeaponOnDeathField(_) => on_death::write(spec, field),
        _ => plain::write(spec, field),
    }
}
