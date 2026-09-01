//! The Weapon form's own fields, one arm per control its def panel draws.

use serde::{Deserialize, Serialize};

use super::draft_name::EditorDraftNameNet;
use crate::net_qa::wire::{
    attachment::{DamageTypeNet, FatalBiasNet, WeaponDamageNet, WeaponPunchNet, WeaponShredNet},
    fire_mode::HitTypeNet,
    list::EditorListIndexNet,
    melee_weapon::{HandednessNet, ShoveNet},
    on_death::OnDeathVariantNet,
    weapon::{
        AccuracyNet, BaseSpreadNet, DotDamageNet, DotEnabledNet, DotTurnsNet, ExplodeDamageNet,
        FieldKeyNet, KickbackNet, MagazineSizeNet, ReloadTuNet, StableNet, TrajectoryStyleNet,
    },
};

/// One field of the Weapon draft, carrying the value it is set to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(in crate::net_qa) enum WeaponFieldNet {
    /// The draft's display name.
    Name(EditorDraftNameNet),
    /// The draft's base cone spread.
    BaseSpread(BaseSpreadNet),
    /// The draft's accuracy.
    Accuracy(AccuracyNet),
    /// The draft's kickback.
    Kickback(KickbackNet),
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
    /// The draft's shot trajectory.
    Trajectory(TrajectoryStyleNet),
    /// Whether the draft is braced by design.
    Stable(StableNet),
    /// Whether the draft shoves on connect.
    Shove(ShoveNet),
    /// The draft's magazine size.
    MagazineSize(MagazineSizeNet),
    /// The draft's reload cost in time units.
    MagazineReloadTu(ReloadTuNet),
    /// Whether the draft authors a damage-over-time profile.
    Dot(DotEnabledNet),
    /// The DOT profile's per-turn damage, behind the DOT tick box.
    DotDamage(DotDamageNet),
    /// The DOT profile's duration, behind the DOT tick box, where the form turns zero into one.
    DotTurns(DotTurnsNet),
    /// The DOT profile's damage channel, behind the DOT tick box.
    DotDamageType(DamageTypeNet),
    /// Which variant the on-death effect at one index is on.
    OnDeathVariant {
        /// Which effect of the list.
        index:   EditorListIndexNet,
        /// The variant its combo is set to.
        variant: OnDeathVariantNet,
    },
    /// The explode effect's hit geometry, at one index.
    OnDeathHitType {
        /// Which effect of the list.
        index:    EditorListIndexNet,
        /// The geometry its row draws.
        hit_type: HitTypeNet,
    },
    /// The explode effect's blast damage, at one index.
    OnDeathDamage {
        /// Which effect of the list.
        index:  EditorListIndexNet,
        /// The flat damage each cell takes.
        damage: ExplodeDamageNet,
    },
    /// The explode effect's damage channel, at one index.
    OnDeathDamageType {
        /// Which effect of the list.
        index:       EditorListIndexNet,
        /// The channel the blast deals on.
        damage_type: DamageTypeNet,
    },
    /// The leave-field effect's field key, at one index.
    OnDeathField {
        /// Which effect of the list.
        index: EditorListIndexNet,
        /// The catalog key of the field left behind.
        field: FieldKeyNet,
    },
}
