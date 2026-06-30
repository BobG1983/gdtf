//! GTW-505 — the melee weapon MODEL tests: the shipped `fists.melee_weapon.ron` parses,
//! a `MeleeWeaponSpec` round-trips, `into_bundle` SHARES the ranged damage components and
//! LACKS the ranged-only ones, and the `MeleeWeaponRegistry` resolves the `fists` default.
//!
//! Value-agnostic on the tunable magnitudes (the authored numbers are DATA, not pinned by
//! tests — the brittle-data rule); the assertions are STRUCTURAL (which components a melee
//! bundle carries / lacks, the marker, the registry key resolution).

use super::support::*;
use crate::weapon::{
    FISTS_KEY, FightModeKind, MeleeWeapon, MeleeWeaponRegistry, MeleeWeaponSpec, Reach,
};

/// A shipped melee weapon `.melee_weapon.ron`, read at compile time via the same
/// `include_str!` pattern the ranged shipped weapon test uses — the REAL on-disk authored
/// file (`assets/content/weapons/melee/fists.melee_weapon.ron`), so a regression in the
/// authored file turns this red.
const SHIPPED_FISTS_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/content/weapons/melee/fists.melee_weapon.ron"
));

/// A shipped TWO-handed high-shred melee weapon — the chainsword (the authored-melee witness).
const SHIPPED_CHAINSWORD_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/content/weapons/melee/chainsword.melee_weapon.ron"
));

/// GTW-505 C6 — the shipped `fists.melee_weapon.ron` parses into a `MeleeWeaponSpec`, and
/// `into_bundle(name)` yields a `MeleeWeaponBundle` carrying that `WeaponName` + the
/// `MeleeWeapon` marker + a `FightMode` list offering at least one mode. STRUCTURAL +
/// value-agnostic: asserts the NAME landed, the marker landed, and the fight-mode offers a
/// mode whose `primary()` is a known `FightModeKind` — never a tunable magnitude.
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

    // The file does NOT author a name — the name is the FILE KEY, supplied here.
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
    // The primary mode is one of the closed FightModeKind variants (mechanism, not a value).
    let primary = bundle.fight_mode.primary();
    assert!(
        matches!(primary.kind, FightModeKind::Swing | FightModeKind::Thrust),
        "the fists primary() mode is a known FightModeKind",
    );
}

/// GTW-505 C6 — a `MeleeWeaponSpec` round-trips from inline RON (no shipped magnitudes) and
/// `into_bundle` groups the damage block + carries the melee-only Reach/FightMode
/// faithfully: a spot value read back off the bundle equals the authored one. Arbitrary
/// literals (mechanism, not a balance pin).
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
    // Spot values flowed through the MeleeDamageProfile grouping + the direct fields
    // (distinct arbitrary literals so a field swap would surface).
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

/// GTW-505 C6 — an omitted `reach:` field defaults to `Reach::DEFAULT` (1): the first-slice
/// "Reach default 1" ruling holds for a weapon that does not author it (the
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

/// GTW-505 C6 — the shipped `chainsword.melee_weapon.ron` parses (the authored-melee witness)
/// and resolves to a `TwoHanded` bundle (enum resolution only — no magnitude pin).
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

/// GTW-505 C6 — a `MeleeWeaponRegistry` keys specs by `WeaponName`, resolves a lookup, and
/// answers the `fists` default: a present key returns the spec, the `fists` accessor finds
/// the fists default, an absent key returns `None`.
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
