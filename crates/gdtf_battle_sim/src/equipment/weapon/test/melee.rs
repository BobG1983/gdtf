//! Melee weapon tests; magnitudes are data, not pinned literals.
use super::support::*;
use crate::weapon::{
    FISTS_KEY, FightModeKind, MeleeWeapon, MeleeWeaponRegistry, MeleeWeaponSpec, Reach,
};

const SHIPPED_FISTS_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/content/weapons/melee/fists.melee_weapon.ron"
));

const SHIPPED_CHAINSWORD_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/content/weapons/melee/chainsword.melee_weapon.ron"
));

#[test]
fn shipped_fists_spec_parses_and_converts_to_a_bundle() {
    let parsed = ron::de::from_str::<MeleeWeaponSpec>(SHIPPED_FISTS_RON);
    assert!(
        parsed.is_ok(),
        "the shipped fists.melee_weapon.ron must parse into a MeleeWeaponSpec: {parsed:?}",
    );
    let Ok(spec) = parsed else {
        return;
    };

    let key = WeaponName::new(FISTS_KEY.to_owned());
    let bundle = spec.into_bundle(key);
    assert_eq!(
        &*bundle.name, FISTS_KEY,
        "into_bundle must carry the supplied WeaponName (the file key) onto the melee bundle",
    );
    assert_eq!(
        bundle.marker, MeleeWeapon,
        "into_bundle adds the MeleeWeapon marker (not the ranged Weapon marker)",
    );
    assert!(
        !bundle.fight_mode.is_empty(),
        "the authored fight-mode must offer at least one mode",
    );
    let primary = bundle.fight_mode.primary();
    assert!(
        matches!(primary.kind, FightModeKind::Swing | FightModeKind::Thrust),
        "the fists primary() mode is a known FightModeKind",
    );
}

#[test]
fn melee_spec_round_trips_and_into_bundle_groups_faithfully() {
    let authored = r"(
        damage: 14, punch: 6, shred: 9, damage_type: Rend, fatal_bias: 5.0,
        handedness: TwoHanded, reach: 2,
        fight_mode: [ ( kind: Swing, tu_cost: 25, strikes: 1 ) ],
    )";
    let parsed = ron::de::from_str::<MeleeWeaponSpec>(authored);
    assert!(
        parsed.is_ok(),
        "inline MeleeWeaponSpec RON must parse: {parsed:?}"
    );
    let Ok(spec) = parsed else {
        return;
    };

    let bundle = spec.into_bundle(WeaponName::new("test-blade".to_owned()));
    assert_eq!(
        *bundle.damage, 14i32,
        "damage flows through MeleeDamageProfile"
    );
    assert_eq!(
        *bundle.punch, 6i32,
        "punch flows through MeleeDamageProfile"
    );
    assert_eq!(
        *bundle.shred, 9i32,
        "shred flows through MeleeDamageProfile"
    );
    assert_eq!(
        *bundle.reach, 2u16,
        "the melee-only Reach flows through into_bundle"
    );
    let kinds: Vec<FightModeKind> = bundle.fight_mode.iter().map(|spec| spec.kind).collect();
    assert_eq!(kinds, vec![FightModeKind::Swing]);
}

/// `#[serde(default)]` on the spec field).
#[test]
fn omitted_reach_defaults_to_one() {
    let authored = r"(
        damage: 4, punch: 0, shred: 0, damage_type: Kinetic, fatal_bias: 0.0,
        handedness: OneHanded,
        fight_mode: [ ( kind: Swing, tu_cost: 20, strikes: 1 ) ],
    )";
    let parsed = ron::de::from_str::<MeleeWeaponSpec>(authored);
    assert!(
        parsed.is_ok(),
        "a reach-less MeleeWeaponSpec must parse: {parsed:?}"
    );
    let Ok(spec) = parsed else {
        return;
    };
    assert_eq!(
        spec.reach,
        Reach::DEFAULT,
        "an omitted reach: field defaults to Reach::DEFAULT (1)",
    );
    assert_eq!(
        *Reach::DEFAULT,
        1u16,
        "precondition: the default reach is 1"
    );
}

#[test]
fn shipped_chainsword_parses_two_handed() {
    let parsed = ron::de::from_str::<MeleeWeaponSpec>(SHIPPED_CHAINSWORD_RON);
    assert!(
        parsed.is_ok(),
        "the shipped chainsword.melee_weapon.ron must parse: {parsed:?}",
    );
    let Ok(spec) = parsed else {
        return;
    };
    let bundle = spec.into_bundle(WeaponName::new("chainsword".to_owned()));
    assert_eq!(
        bundle.handedness,
        crate::weapon::Handedness::TwoHanded,
        "the shipped chainsword is a two-handed weapon",
    );
}

#[test]
fn melee_registry_keys_resolves_and_answers_fists() {
    let Ok(fists) = ron::de::from_str::<MeleeWeaponSpec>(SHIPPED_FISTS_RON) else {
        return;
    };
    let registry = MeleeWeaponRegistry::new([(WeaponName::new(FISTS_KEY.to_owned()), fists)]);

    assert_eq!(
        registry.len(),
        1,
        "the registry holds the one inserted melee weapon"
    );
    assert!(!registry.is_empty(), "a one-weapon registry is non-empty");
    assert!(
        registry
            .spec(&WeaponName::new(FISTS_KEY.to_owned()))
            .is_some(),
        "a present key resolves to its spec",
    );
    assert!(
        registry.fists().is_some(),
        "the fists() accessor resolves the shipped default (the None -> fists fallback source)",
    );
    assert!(
        registry
            .spec(&WeaponName::new("missing".to_owned()))
            .is_none(),
        "an absent key resolves to None (the setup-time MeleeWeaponNotFound trigger)",
    );
}
