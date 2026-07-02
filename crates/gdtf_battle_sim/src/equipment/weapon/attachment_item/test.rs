//! Unit tests for the GTW-549 PHASE 1 attachment-item model: the closed
//! [`AttachmentEffect`](super::AttachmentEffect) vocabulary parses from RON by variant
//! name (the serde bridge), the [`AttachmentSpec`](super::AttachmentSpec) parses (identity
//! + effect list), and the [`AttachmentRegistry`](super::AttachmentRegistry) keys by name.
//!
//! Per the brittle-test rule (data `.ron` is tunable), these assert the MECHANISM — that
//! the schema + file agree and that each variant round-trips — NOT any shipped magnitude.

use super::{AttachmentEffect, AttachmentName, AttachmentRegistry, AttachmentSpec};
use crate::weapon::{DamageType, WeaponName};

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

/// An [`AttachmentSpec`] parses from a RON item carrying a `display_name` + an `effects`
/// list of mixed variants. Pin-discriminating: a schema mismatch fails to parse. Asserts
/// the list is populated (mechanism), not the magnitudes.
#[test]
fn attachment_spec_parses_with_effect_list() {
    let ron = "(display_name: \"Whisper Bore\", effects: [Silence, Aim(0.2)])";
    let Ok(spec) = ron::de::from_str::<AttachmentSpec>(ron) else {
        unreachable!("an AttachmentSpec with a display_name + effects list must parse");
    };
    assert_eq!(
        spec.display_name,
        WeaponName::new("Whisper Bore".to_owned()),
        "the display_name round-trips"
    );
    assert_eq!(
        spec.effects.len(),
        2,
        "both authored effects parse into the list"
    );
    assert!(
        matches!(spec.effects.first(), Some(AttachmentEffect::Silence)),
        "the first effect is the no-payload Silence"
    );
}

/// A cosmetic [`AttachmentSpec`] that authors NO `effects:` field parses to an EMPTY list
/// (the `#[serde(default)]` identity) — a weapon fitting it applies nothing.
#[test]
fn attachment_spec_defaults_to_empty_effects() {
    let Ok(spec) = ron::de::from_str::<AttachmentSpec>("(display_name: \"Bare Rail\")") else {
        unreachable!("an AttachmentSpec omitting `effects:` must parse (serde default)");
    };
    assert!(
        spec.effects.is_empty(),
        "an omitted effects list defaults to empty (identity)"
    );
}

/// The [`AttachmentRegistry`] keys each spec by its [`AttachmentName`] and answers a
/// lookup — the shape the folder loader (and PHASE 2 setup) rely on. A missing key returns
/// [`None`] (fail-closed). Asserts the map mechanism, not any content.
#[test]
fn registry_keys_and_looks_up_by_name() {
    let Ok(spec) = ron::de::from_str::<AttachmentSpec>(
        "(display_name: \"Scoped Sight\", effects: [Aim(0.4)])",
    ) else {
        unreachable!("the fixture spec must parse");
    };
    let key = AttachmentName::new("scoped_sight".to_owned());
    let registry = AttachmentRegistry::new([(key.clone(), spec)]);

    assert_eq!(
        registry.len(),
        1,
        "the registry holds the one inserted item"
    );
    assert!(
        registry.spec(&key).is_some(),
        "an inserted key resolves to its spec"
    );
    assert!(
        registry
            .spec(&AttachmentName::new("missing".to_owned()))
            .is_none(),
        "a missing key fails closed with None"
    );
}
