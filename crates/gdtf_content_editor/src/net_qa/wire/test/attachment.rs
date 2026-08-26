use gdtf_battle_sim::{
    effects::attachments::{AimDelta, AttachmentEffect, ReloadTimeScale, WeaponBraceBonus},
    equipment::attachments::AttachmentSlot,
    weapon::{
        DamageType, FatalBias, FireModeSpec, MagazineSize, ModeConeMult, ModeKind, ModeShots,
        ModeTuPercent, WeaponDamage, WeaponPunch, WeaponShred,
    },
};

use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::net_qa::wire::{
    AttachmentEffectNet, AttachmentSlotNet,
    attachment::{
        AimDeltaNet, BraceBonusNet, DamageTypeNet, FatalBiasNet, MagazineSizeNet,
        ReloadTimeScaleNet, WeaponDamageNet, WeaponPunchNet, WeaponShredNet,
    },
};

/// One effect per arm the form's own variant picker offers.
fn every_effect() -> [AttachmentEffect; 13] {
    [
        AttachmentEffect::Aim(AimDelta::new(0.25)),
        AttachmentEffect::Stability(WeaponBraceBonus::new(1.5)),
        AttachmentEffect::GainFireMode(FireModeSpec::new(
            ModeKind::Burst,
            ModeConeMult::new(1.0),
            ModeTuPercent::new(0.4),
            ModeShots::new(3),
        )),
        AttachmentEffect::ExtraAmmo(MagazineSize::new(6)),
        AttachmentEffect::ReloadTime(ReloadTimeScale::new(0.8)),
        AttachmentEffect::Silence,
        AttachmentEffect::Penetration(WeaponPunch::new(2)),
        AttachmentEffect::DamageTypeOverride(DamageType::Plasma),
        AttachmentEffect::Damage(WeaponDamage::new(4)),
        AttachmentEffect::Shred(WeaponShred::new(5)),
        AttachmentEffect::FatalBias(FatalBias::new(0.1)),
        AttachmentEffect::Brace,
        AttachmentEffect::Shove,
    ]
}

#[test]
fn every_slot_round_trips_and_reads_back_as_the_slot_it_mirrored() {
    for slot in AttachmentSlot::ALL {
        let mirrored = AttachmentSlotNet::from_slot(slot);
        assert_ron_round_trip(&mirrored);
        assert_eq!(
            mirrored.to_slot(),
            slot,
            "a client's slot must come back as the sim's own",
        );
    }
}

#[test]
fn every_effect_arm_round_trips_and_reads_back_with_its_payload() {
    for effect in every_effect() {
        let mirrored = AttachmentEffectNet::from_effect(&effect);
        assert_ron_round_trip(&mirrored);
        assert_eq!(
            mirrored.to_effect(),
            effect,
            "a client's effect must come back as the sim's own, payload and all",
        );
    }
}

#[test]
fn every_effect_arm_carries_its_own_name_on_the_wire() {
    let mut seen: Vec<String> = Vec::with_capacity(every_effect().len());
    for effect in every_effect() {
        let mirrored = format!("{:?}", AttachmentEffectNet::from_effect(&effect));
        assert!(
            !seen.contains(&mirrored),
            "{effect:?} maps onto {mirrored}, which another effect already claims — two effects \
             that read the same on the wire are indistinguishable to a client",
        );
        seen.push(mirrored);
    }
}

#[test]
fn every_damage_type_round_trips_and_reads_back_as_the_channel_it_mirrored() {
    for damage_type in DamageType::ALL {
        let mirrored = DamageTypeNet::from_damage_type(damage_type);
        assert_ron_round_trip(&mirrored);
        assert_eq!(
            mirrored.to_damage_type(),
            damage_type,
            "a client's damage channel must come back as the sim's own",
        );
    }
}

#[test]
fn every_effect_payload_round_trips_on_its_own() {
    assert_ron_round_trip(&AimDeltaNet::new(0.25));
    assert_ron_round_trip(&BraceBonusNet::new(1.5));
    assert_ron_round_trip(&MagazineSizeNet::new(6));
    assert_ron_round_trip(&ReloadTimeScaleNet::new(0.8));
    assert_ron_round_trip(&WeaponPunchNet::new(2));
    assert_ron_round_trip(&WeaponDamageNet::new(4));
    assert_ron_round_trip(&WeaponShredNet::new(5));
    assert_ron_round_trip(&FatalBiasNet::new(0.1));
}

#[test]
fn the_attachment_mirrors_trace_usable_shapes() {
    assert_schema_is_usable::<AttachmentSlotNet>("AttachmentSlotNet");
    assert_schema_is_usable::<AttachmentEffectNet>("AttachmentEffectNet");
    assert_schema_is_usable::<DamageTypeNet>("DamageTypeNet");
    assert_schema_is_usable::<AimDeltaNet>("AimDeltaNet");
    assert_schema_is_usable::<BraceBonusNet>("BraceBonusNet");
    assert_schema_is_usable::<MagazineSizeNet>("MagazineSizeNet");
    assert_schema_is_usable::<ReloadTimeScaleNet>("ReloadTimeScaleNet");
    assert_schema_is_usable::<WeaponPunchNet>("WeaponPunchNet");
    assert_schema_is_usable::<WeaponDamageNet>("WeaponDamageNet");
    assert_schema_is_usable::<WeaponShredNet>("WeaponShredNet");
    assert_schema_is_usable::<FatalBiasNet>("FatalBiasNet");
}
