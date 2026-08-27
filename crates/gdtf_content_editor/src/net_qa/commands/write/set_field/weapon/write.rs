//! Route one Weapon field arm to the widget group that draws it.

use super::{dot, on_death, plain};
use crate::{
    net_qa::{
        commands::write::form_fault::FormWriteFault,
        wire::{EditorDraftNameNet, WeaponFieldNet},
    },
    weapon_form::WeaponDraft,
};

// The save stem is the one row the draft owns rather than the spec.
fn write_name(draft: &mut WeaponDraft, name: &EditorDraftNameNet) -> WeaponFieldNet {
    draft.set_name((**name).clone());
    WeaponFieldNet::Name(EditorDraftNameNet::new(draft.name()))
}

/// Write one Weapon field, answering the field as the draft stores it.
pub(in crate::net_qa::commands::write::set_field) fn write(
    draft: &mut WeaponDraft,
    field: WeaponFieldNet,
) -> Result<WeaponFieldNet, FormWriteFault> {
    match field {
        WeaponFieldNet::Name(name) => Ok(write_name(draft, &name)),
        WeaponFieldNet::BaseSpread(spread) => Ok(plain::base_spread(draft.spec_mut(), spread)),
        WeaponFieldNet::Accuracy(accuracy) => Ok(plain::accuracy(draft.spec_mut(), accuracy)),
        WeaponFieldNet::Kickback(kickback) => Ok(plain::kickback(draft.spec_mut(), kickback)),
        WeaponFieldNet::Damage(damage) => Ok(plain::damage(draft.spec_mut(), damage)),
        WeaponFieldNet::Punch(punch) => Ok(plain::punch(draft.spec_mut(), punch)),
        WeaponFieldNet::Shred(shred) => Ok(plain::shred(draft.spec_mut(), shred)),
        WeaponFieldNet::DamageType(channel) => Ok(plain::damage_type(draft.spec_mut(), channel)),
        WeaponFieldNet::FatalBias(bias) => Ok(plain::fatal_bias(draft.spec_mut(), bias)),
        WeaponFieldNet::Handedness(handedness) => {
            Ok(plain::handedness(draft.spec_mut(), handedness))
        }
        WeaponFieldNet::Trajectory(trajectory) => {
            Ok(plain::trajectory(draft.spec_mut(), trajectory))
        }
        WeaponFieldNet::Stable(stable) => Ok(plain::stable(draft.spec_mut(), stable)),
        WeaponFieldNet::Shove(shove) => Ok(plain::shove(draft.spec_mut(), shove)),
        WeaponFieldNet::MagazineSize(size) => Ok(plain::magazine_size(draft.spec_mut(), size)),
        WeaponFieldNet::MagazineReloadTu(reload) => {
            Ok(plain::magazine_reload_tu(draft.spec_mut(), reload))
        }
        WeaponFieldNet::Dot(enabled) => Ok(dot::enabled(draft.spec_mut(), enabled)),
        WeaponFieldNet::DotDamage(damage) => dot::damage(draft.spec_mut(), damage),
        WeaponFieldNet::DotTurns(turns) => dot::turns(draft.spec_mut(), turns),
        WeaponFieldNet::DotDamageType(channel) => dot::damage_type(draft.spec_mut(), channel),
        WeaponFieldNet::OnDeath(enabled) => Ok(on_death::enabled(draft.spec_mut(), enabled)),
        WeaponFieldNet::OnDeathVariant(wanted) => on_death::variant(draft.spec_mut(), wanted),
        WeaponFieldNet::OnDeathHitType(wanted) => on_death::hit_type(draft.spec_mut(), wanted),
        WeaponFieldNet::OnDeathDamage(wanted) => on_death::damage(draft.spec_mut(), wanted),
        WeaponFieldNet::OnDeathDamageType(wanted) => {
            on_death::damage_type(draft.spec_mut(), wanted)
        }
        WeaponFieldNet::OnDeathField(wanted) => on_death::field(draft.spec_mut(), wanted),
    }
}
