//! Palette-level behaviour suite for the GTW-558 attachment-effect palette: the closed
//! [`AttachmentEffect`](super::AttachmentEffect) vocabulary parses from RON by variant name
//! (the serde bridge), and the enum's THIN delegation `impl ApplyAttachmentEffect` routes each
//! variant to its isolated `ApplyX` behaviour (applied directly through the trait against an
//! [`EntityWorldMut`], the surface the mechanics' commands extension wraps).
//!
//! Per-effect mapping/direction is asserted in each effect file's own `#[cfg(test)]`; this
//! suite proves the enum bridge (parse + delegation), not the individual stat maths. Per the
//! brittle-test rule, assertions check the MAPPING + DIRECTION, never a shipped magnitude.

use bevy::prelude::World;

use super::{AimDelta, ApplyAttachmentEffect, AttachmentEffect};
use crate::weapon::{Accuracy, DamageType};

/// The closed [`AttachmentEffect`] vocabulary deserializes each variant from RON by name,
/// with its per-item magnitude payload authored as a bare scalar (the `#[serde(transparent)]`
/// newtype bridge). Pin-discriminating: a mis-named variant or a wrong payload shape fails
/// to parse. Asserts the MAPPING (which variant / which payload type), not a magnitude.
#[test]
fn each_effect_variant_parses_from_ron() {
    // Aim carries a bare AimDelta scalar.
    let Ok(aim) = ron::de::from_str::<AttachmentEffect>("Aim(0.4)") else {
        unreachable!("Aim(<f32>) must parse");
    };
    assert!(
        matches!(aim, AttachmentEffect::Aim(_)),
        "Aim maps to the Aim variant"
    );

    // Stability carries a bare WeaponBraceBonus scalar.
    let Ok(stability) = ron::de::from_str::<AttachmentEffect>("Stability(12.0)") else {
        unreachable!("Stability(<f32>) must parse");
    };
    assert!(
        matches!(stability, AttachmentEffect::Stability(_)),
        "Stability maps to the Stability variant"
    );

    // ReloadTime carries a bare ReloadTimeScale scalar.
    let Ok(reload) = ron::de::from_str::<AttachmentEffect>("ReloadTime(1.5)") else {
        unreachable!("ReloadTime(<f32>) must parse");
    };
    assert!(
        matches!(reload, AttachmentEffect::ReloadTime(_)),
        "ReloadTime maps to the ReloadTime variant"
    );

    // ExtraAmmo carries a bare MagazineSize scalar.
    let Ok(ammo) = ron::de::from_str::<AttachmentEffect>("ExtraAmmo(6)") else {
        unreachable!("ExtraAmmo(<u16>) must parse");
    };
    assert!(
        matches!(ammo, AttachmentEffect::ExtraAmmo(_)),
        "ExtraAmmo maps to the ExtraAmmo variant"
    );

    // GainFireMode carries a full FireModeSpec.
    let Ok(mode) = ron::de::from_str::<AttachmentEffect>(
        "GainFireMode((kind: Burst, cone_mult: 1.3, tu_percent: 0.5, shots: 3))",
    ) else {
        unreachable!("GainFireMode(<FireModeSpec>) must parse");
    };
    assert!(
        matches!(mode, AttachmentEffect::GainFireMode(_)),
        "GainFireMode maps to the GainFireMode variant"
    );

    // DamageTypeOverride carries a DamageType enum variant.
    let Ok(over) = ron::de::from_str::<AttachmentEffect>("DamageTypeOverride(Chem)") else {
        unreachable!("DamageTypeOverride(<DamageType>) must parse");
    };
    assert_eq!(
        over,
        AttachmentEffect::DamageTypeOverride(DamageType::Chem),
        "DamageTypeOverride carries the named damage type"
    );

    // The no-payload variants parse as bare unit variants.
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

/// The [`AttachmentEffect`] enum's THIN delegation `impl ApplyAttachmentEffect` routes a
/// variant to its isolated behaviour — applying `Aim` through the enum mutates `Accuracy`
/// exactly as the isolated `ApplyAim` does. Proves the bridge forwards, not that the enum
/// carries logic.
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
