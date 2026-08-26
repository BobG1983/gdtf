//! The Melee Weapon form's own field arms, written through the spec its def panel writes.

use gdtf_battle_sim::weapon::{FatalBias, Reach, WeaponDamage, WeaponPunch, WeaponShred};

use crate::{
    melee_weapon_form::MeleeWeaponDraft,
    net_qa::{
        commands::write::form_fault::FormWriteFault,
        wire::{
            DamageTypeNet, EditorDraftNameNet, EditorFieldNet, FatalBiasNet, HandednessNet,
            ReachNet, ShoveNet, WeaponDamageNet, WeaponPunchNet, WeaponShredNet,
        },
    },
};

/// Write one Melee Weapon field, answering the field as the draft stores it.
pub(super) fn write(
    draft: &mut MeleeWeaponDraft,
    field: EditorFieldNet,
) -> Result<EditorFieldNet, FormWriteFault> {
    match field {
        EditorFieldNet::MeleeWeaponName(name) => {
            draft.set_name((*name).clone());
            Ok(EditorFieldNet::MeleeWeaponName(EditorDraftNameNet::new(
                draft.name(),
            )))
        }
        EditorFieldNet::MeleeWeaponDamage(damage) => {
            draft.spec_mut().damage = WeaponDamage::new(*damage);
            Ok(EditorFieldNet::MeleeWeaponDamage(WeaponDamageNet::new(
                *draft.spec().damage,
            )))
        }
        EditorFieldNet::MeleeWeaponPunch(punch) => {
            draft.spec_mut().punch = WeaponPunch::new(*punch);
            Ok(EditorFieldNet::MeleeWeaponPunch(WeaponPunchNet::new(
                *draft.spec().punch,
            )))
        }
        EditorFieldNet::MeleeWeaponShred(shred) => {
            draft.spec_mut().shred = WeaponShred::new(*shred);
            Ok(EditorFieldNet::MeleeWeaponShred(WeaponShredNet::new(
                *draft.spec().shred,
            )))
        }
        EditorFieldNet::MeleeWeaponDamageType(damage_type) => {
            draft.spec_mut().damage_type = damage_type.to_damage_type();
            Ok(EditorFieldNet::MeleeWeaponDamageType(
                DamageTypeNet::from_damage_type(draft.spec().damage_type),
            ))
        }
        EditorFieldNet::MeleeWeaponFatalBias(bias) => {
            draft.spec_mut().fatal_bias = FatalBias::new(*bias);
            Ok(EditorFieldNet::MeleeWeaponFatalBias(FatalBiasNet::new(
                *draft.spec().fatal_bias,
            )))
        }
        EditorFieldNet::MeleeWeaponHandedness(handedness) => {
            draft.spec_mut().handedness = handedness.to_handedness();
            Ok(EditorFieldNet::MeleeWeaponHandedness(
                HandednessNet::from_handedness(draft.spec().handedness),
            ))
        }
        EditorFieldNet::MeleeWeaponReach(reach) => {
            draft.spec_mut().reach = Reach::new((*reach).max(1));
            Ok(EditorFieldNet::MeleeWeaponReach(ReachNet::from_reach(
                draft.spec().reach,
            )))
        }
        EditorFieldNet::MeleeWeaponShove(shove) => {
            draft.spec_mut().shove = shove.to_shove();
            Ok(EditorFieldNet::MeleeWeaponShove(ShoveNet::new(
                *draft.spec().shove,
            )))
        }
        _ => Err(FormWriteFault::ForeignArm),
    }
}
