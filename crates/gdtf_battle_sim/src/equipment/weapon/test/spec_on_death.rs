//! `WeaponSpec::into_bundle` turns the authored on-death list into the sibling component.

use super::support::*;
use crate::{
    effects::{
        fields::FieldKey,
        on_death::{ExplodeDamage, OnDeathEffect},
    },
    test_support::test_weapon_spec,
    weapon::BlastRadius,
};

fn explode() -> OnDeathEffect {
    OnDeathEffect::Explode {
        hit_type:    HitType::Blast {
            radius: BlastRadius::new(1),
        },
        damage:      ExplodeDamage::new(8),
        damage_type: DamageType::Blast,
    }
}

fn leave_burning() -> OnDeathEffect {
    OnDeathEffect::LeaveField {
        field: FieldKey::new("burning".to_owned()),
    }
}

#[test]
fn a_spec_authoring_no_on_death_effect_attaches_no_component() {
    let spec = WeaponSpec {
        on_death: Vec::new(),
        ..test_weapon_spec()
    };

    let siblings = spec.into_bundle(WeaponName::new("test-gun".to_owned())).1;

    assert!(
        siblings.on_death().is_none(),
        "an empty authored list attaches no OnDeath component, got {:?}",
        siblings.on_death(),
    );
}

#[test]
fn a_spec_authoring_two_effects_carries_both_in_order() {
    let spec = WeaponSpec {
        on_death: vec![explode(), leave_burning()],
        ..test_weapon_spec()
    };

    let siblings = spec.into_bundle(WeaponName::new("test-gun".to_owned())).1;

    let Some(on_death) = siblings.on_death() else {
        unreachable!("a spec authoring two effects attaches an OnDeath component");
    };
    assert_eq!(
        on_death.effects(),
        [explode(), leave_burning()],
        "the component carries both authored effects, in the authored order",
    );
}
