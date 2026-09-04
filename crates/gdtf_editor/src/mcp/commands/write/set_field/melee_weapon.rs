//! The Melee Weapon form's own field arms, written through the spec its def panel writes.

use gdtf_battle_sim::weapon::{FatalBias, Reach, WeaponDamage, WeaponPunch, WeaponShred};

use crate::{
    mcp::wire::{
        DamageTypeNet, EditorDraftNameNet, FatalBiasNet, HandednessNet, MeleeWeaponFieldNet,
        ReachNet, ShoveNet, WeaponDamageNet, WeaponPunchNet, WeaponShredNet,
    },
    melee_weapon_form::MeleeWeaponDraft,
};

/// Write one Melee Weapon field, answering the field as the draft stores it. The form draws
/// every row ungated, so no write of one is refused.
pub(super) fn write(
    draft: &mut MeleeWeaponDraft,
    field: MeleeWeaponFieldNet,
) -> MeleeWeaponFieldNet {
    match field {
        MeleeWeaponFieldNet::Name(name) => {
            draft.set_name((*name).clone());
            MeleeWeaponFieldNet::Name(EditorDraftNameNet::new(draft.name()))
        }
        MeleeWeaponFieldNet::Damage(damage) => {
            draft.spec_mut().damage = WeaponDamage::new(*damage);
            MeleeWeaponFieldNet::Damage(WeaponDamageNet::new(*draft.spec().damage))
        }
        MeleeWeaponFieldNet::Punch(punch) => {
            draft.spec_mut().punch = WeaponPunch::new(*punch);
            MeleeWeaponFieldNet::Punch(WeaponPunchNet::new(*draft.spec().punch))
        }
        MeleeWeaponFieldNet::Shred(shred) => {
            draft.spec_mut().shred = WeaponShred::new(*shred);
            MeleeWeaponFieldNet::Shred(WeaponShredNet::new(*draft.spec().shred))
        }
        MeleeWeaponFieldNet::DamageType(damage_type) => {
            draft.spec_mut().damage_type = damage_type.to_damage_type();
            MeleeWeaponFieldNet::DamageType(DamageTypeNet::from_damage_type(
                draft.spec().damage_type,
            ))
        }
        MeleeWeaponFieldNet::FatalBias(bias) => {
            draft.spec_mut().fatal_bias = FatalBias::new(*bias);
            MeleeWeaponFieldNet::FatalBias(FatalBiasNet::new(*draft.spec().fatal_bias))
        }
        MeleeWeaponFieldNet::Handedness(handedness) => {
            draft.spec_mut().handedness = handedness.to_handedness();
            MeleeWeaponFieldNet::Handedness(HandednessNet::from_handedness(draft.spec().handedness))
        }
        MeleeWeaponFieldNet::Reach(reach) => {
            draft.spec_mut().reach = Reach::new((*reach).max(1));
            MeleeWeaponFieldNet::Reach(ReachNet::from_reach(draft.spec().reach))
        }
        MeleeWeaponFieldNet::Shove(shove) => {
            draft.spec_mut().shove = shove.to_shove();
            MeleeWeaponFieldNet::Shove(ShoveNet::new(*draft.spec().shove))
        }
    }
}
