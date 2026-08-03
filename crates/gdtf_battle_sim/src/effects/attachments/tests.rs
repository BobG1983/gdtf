//! Per-effect mapping/direction is asserted in each effect file's own `#[cfg(test)]`; this
use bevy::prelude::World;

use super::{AimDelta, ApplyAttachmentEffect, AttachmentEffect};
use crate::weapon::{Accuracy, DamageType};

/// with its per-item magnitude payload authored as a bare scalar (the `#[serde(transparent)]`
#[test]
fn each_effect_variant_parses_from_ron() {
    let Ok(aim) = ron::de::from_str::<AttachmentEffect>("Aim(0.4)") else {
        unreachable!("Aim(<f32>) must parse");
    };
    assert!(
        matches!(aim, AttachmentEffect::Aim(_)),
        "Aim maps to the Aim variant"
    );

    let Ok(stability) = ron::de::from_str::<AttachmentEffect>("Stability(12.0)") else {
        unreachable!("Stability(<f32>) must parse");
    };
    assert!(
        matches!(stability, AttachmentEffect::Stability(_)),
        "Stability maps to the Stability variant"
    );

    let Ok(reload) = ron::de::from_str::<AttachmentEffect>("ReloadTime(1.5)") else {
        unreachable!("ReloadTime(<f32>) must parse");
    };
    assert!(
        matches!(reload, AttachmentEffect::ReloadTime(_)),
        "ReloadTime maps to the ReloadTime variant"
    );

    let Ok(ammo) = ron::de::from_str::<AttachmentEffect>("ExtraAmmo(6)") else {
        unreachable!("ExtraAmmo(<u16>) must parse");
    };
    assert!(
        matches!(ammo, AttachmentEffect::ExtraAmmo(_)),
        "ExtraAmmo maps to the ExtraAmmo variant"
    );

    let Ok(mode) = ron::de::from_str::<AttachmentEffect>(
        "GainFireMode((kind: Burst, cone_mult: 1.3, tu_percent: 0.5, shots: 3))",
    ) else {
        unreachable!("GainFireMode(<FireModeSpec>) must parse");
    };
    assert!(
        matches!(mode, AttachmentEffect::GainFireMode(_)),
        "GainFireMode maps to the GainFireMode variant"
    );

    let Ok(over) = ron::de::from_str::<AttachmentEffect>("DamageTypeOverride(Chem)") else {
        unreachable!("DamageTypeOverride(<DamageType>) must parse");
    };
    assert_eq!(
        over,
        AttachmentEffect::DamageTypeOverride(DamageType::Chem),
        "DamageTypeOverride carries the named damage type"
    );

    for (ron, matches_variant) in [
        (
            "Silence",
            matches!(
                ron::de::from_str::<AttachmentEffect>("Silence"),
                Ok(AttachmentEffect::Silence)
            ),
        ),
        (
            "Brace",
            matches!(
                ron::de::from_str::<AttachmentEffect>("Brace"),
                Ok(AttachmentEffect::Brace)
            ),
        ),
        (
            "Shove",
            matches!(
                ron::de::from_str::<AttachmentEffect>("Shove"),
                Ok(AttachmentEffect::Shove)
            ),
        ),
    ] {
        assert!(
            matches_variant,
            "the no-payload `{ron}` variant must parse as a unit variant"
        );
    }
}

#[test]
fn enum_delegates_to_the_isolated_behaviour() {
    let mut world = World::new();
    let weapon = world.spawn(Accuracy::new(1.0)).id();
    let mut entity = world.entity_mut(weapon);
    AttachmentEffect::Aim(AimDelta::new(0.4)).apply_to_weapon(&mut entity);
    let Some(accuracy) = entity.get::<Accuracy>() else {
        unreachable!("the weapon must still carry Accuracy");
    };
    assert!(
        **accuracy > 1.0,
        "the enum delegates Aim to ApplyAim, raising Accuracy above the 1.0 baseline (got {})",
        **accuracy
    );
}
