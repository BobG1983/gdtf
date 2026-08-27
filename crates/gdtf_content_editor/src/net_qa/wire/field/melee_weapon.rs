//! The Melee Weapon form's own fields, one arm per control its def panel draws.

use serde::{Deserialize, Serialize};

use super::draft_name::EditorDraftNameNet;
use crate::net_qa::wire::{
    attachment::{DamageTypeNet, FatalBiasNet, WeaponDamageNet, WeaponPunchNet, WeaponShredNet},
    melee_weapon::{HandednessNet, ReachNet, ShoveNet},
};

/// One field of the Melee Weapon draft, carrying the value it is set to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(in crate::net_qa) enum MeleeWeaponFieldNet {
    /// The draft's display name.
    Name(EditorDraftNameNet),
    /// The draft's damage.
    Damage(WeaponDamageNet),
    /// The draft's punch.
    Punch(WeaponPunchNet),
    /// The draft's shred.
    Shred(WeaponShredNet),
    /// The draft's damage channel.
    DamageType(DamageTypeNet),
    /// The draft's fatal bias.
    FatalBias(FatalBiasNet),
    /// The draft's handedness.
    Handedness(HandednessNet),
    /// The draft's reach, which the form clamps at one.
    Reach(ReachNet),
    /// Whether the draft shoves on connect.
    Shove(ShoveNet),
}
