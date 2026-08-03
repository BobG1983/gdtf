//! Relocated tests for the `WeaponSpec` authoring struct + the `WeaponRegistry`
//! (GTW-201; moved VERBATIM — was the `=== GTW-257 ===` block of the flat module).

use super::support::*;

/// pattern `tuning.rs` / `situation.rs` use — the REAL on-disk authored file
/// (`assets/content/weapons/ranged/stub_pistol.weapon.ron`), so a regression in the authored file
const SHIPPED_STUB_PISTOL_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/content/weapons/ranged/stub_pistol.weapon.ron"
));

const SHIPPED_LAS_CARBINE_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/content/weapons/ranged/las_carbine.weapon.ron"
));

#[test]
fn shipped_weapon_spec_parses_and_converts_to_a_bundle() {
    let parsed = ron::de::from_str::<WeaponSpec>(SHIPPED_STUB_PISTOL_RON);
    assert!(
        parsed.is_ok(),
        "the shipped assets/content/weapons/ranged/stub_pistol.weapon.ron must parse into a WeaponSpec: {parsed:?}",
    );
    let Ok(spec) = parsed else {
        return;
    };

    let key = WeaponName::new("stub_pistol".to_owned());
    let bundle = spec.into_bundle(key).0;
    assert_eq!(
        &*bundle.name, "stub_pistol",
        "into_bundle must carry the supplied WeaponName (the file key) onto the bundle",
    );
    assert_eq!(bundle.marker, Weapon, "into_bundle adds the Weapon marker");
    assert!(
        !bundle.fire_mode.is_empty(),
        "the authored fire-mode must offer at least one mode",
    );
    let single = bundle.fire_mode.single();
    assert_eq!(
        single.kind,
        ModeKind::Single,
        "the shipped stub_pistol's single() mode is Single-kind",
    );
}

#[test]
fn shipped_weapon_handedness_parses_onto_the_bundle() {
    let carbine = ron::de::from_str::<WeaponSpec>(SHIPPED_LAS_CARBINE_RON);
    assert!(
        carbine.is_ok(),
        "the shipped las_carbine.weapon.ron must parse into a WeaponSpec: {carbine:?}",
    );
    let Ok(carbine) = carbine else { return };
    let carbine_bundle = carbine
        .into_bundle(WeaponName::new("las_carbine".to_owned()))
        .0;
    assert_eq!(
        carbine_bundle.handedness,
        Handedness::TwoHanded,
        "a shipped two-handed long-arm parses to Handedness::TwoHanded",
    );

    let pistol = ron::de::from_str::<WeaponSpec>(SHIPPED_STUB_PISTOL_RON);
    assert!(
        pistol.is_ok(),
        "the shipped stub_pistol.weapon.ron must parse into a WeaponSpec: {pistol:?}",
    );
    let Ok(pistol) = pistol else { return };
    let pistol_bundle = pistol
        .into_bundle(WeaponName::new("stub_pistol".to_owned()))
        .0;
    assert_eq!(
        pistol_bundle.handedness,
        Handedness::OneHanded,
        "a shipped one-handed pistol parses to Handedness::OneHanded",
    );
}

#[test]
fn weapon_spec_round_trips_and_into_bundle_groups_faithfully() {
    let authored = r"(
        base_spread: 0.2, accuracy: 1.1, kickback: 0.3, fatal_bias: 5.0,
        damage: 14, punch: 6, shred: 4, damage_type: Kinetic,
        magazine: ( size: 24, reload_tu: 11 ),
        fire_mode: [
            ( kind: Single, cone_mult: 1.0, tu_percent: 0.5, shots: 1),
            ( kind: Burst,  cone_mult: 1.3, tu_percent: 0.8, shots: 3),
        ],
        stable: false,
        handedness: OneHanded,
    )";
    let parsed = ron::de::from_str::<WeaponSpec>(authored);
    assert!(
        parsed.is_ok(),
        "inline WeaponSpec RON must parse: {parsed:?}"
    );
    let Ok(spec) = parsed else {
        return;
    };

    let bundle = spec.into_bundle(WeaponName::new("test-gun".to_owned())).0;
    assert_eq!(*bundle.damage, 14i32, "damage flows through DamageProfile");
    assert_eq!(*bundle.punch, 6i32, "punch flows through DamageProfile");
    assert_eq!(*bundle.shred, 4i32, "shred flows through DamageProfile");
    assert_eq!(bundle.damage_type, DamageType::Kinetic);
    assert_eq!(
        *bundle.magazine.size(),
        24u16,
        "the magazine size flows through HandlingProfile",
    );
    assert_eq!(
        *bundle.magazine.rounds(),
        *bundle.magazine.size(),
        "into_bundle spawns the magazine full (loaded == size)",
    );
    assert!(!*bundle.stable, "stable flows through HandlingProfile");
    let kinds: Vec<ModeKind> = bundle.fire_mode.iter().map(|spec| spec.kind).collect();
    assert_eq!(kinds, vec![ModeKind::Single, ModeKind::Burst]);
}

#[test]
fn weapon_registry_keys_and_iter_enumerate_all_entries() {
    let minimal_ron = r"(
        base_spread: 0.1, accuracy: 1.0, kickback: 0.1, fatal_bias: 1.0,
        damage: 1, punch: 1, shred: 0, damage_type: Kinetic,
        magazine: ( size: 8, reload_tu: 10 ),
        fire_mode: [ ( kind: Single, cone_mult: 1.0, tu_percent: 0.5, shots: 1 ) ],
        stable: false,
        handedness: OneHanded,
    )";
    let Ok(spec_a) = ron::de::from_str::<WeaponSpec>(minimal_ron) else {
        return;
    };
    let Ok(spec_b) = ron::de::from_str::<WeaponSpec>(minimal_ron) else {
        return;
    };
    let Ok(spec_c) = ron::de::from_str::<WeaponSpec>(minimal_ron) else {
        return;
    };

    let name_a = WeaponName::new("alpha".to_owned());
    let name_b = WeaponName::new("bravo".to_owned());
    let name_c = WeaponName::new("charlie".to_owned());

    let registry = WeaponRegistry::new([
        (name_a.clone(), spec_a),
        (name_b.clone(), spec_b),
        (name_c.clone(), spec_c),
    ]);

    let all_keys: Vec<&WeaponName> = registry.keys().collect();
    assert_eq!(all_keys.len(), 3, "keys() must yield exactly 3 entries");
    assert!(
        all_keys.contains(&&name_a),
        "keys() must include name_a (\"alpha\")"
    );
    assert!(
        all_keys.contains(&&name_b),
        "keys() must include name_b (\"bravo\")"
    );
    assert!(
        all_keys.contains(&&name_c),
        "keys() must include name_c (\"charlie\")"
    );

    let all_pairs: Vec<(&WeaponName, &WeaponSpec)> = registry.iter().collect();
    assert_eq!(
        all_pairs.len(),
        3,
        "iter() must yield exactly 3 (key, spec) pairs"
    );
    let iter_keys: Vec<&WeaponName> = all_pairs.iter().map(|(k, _)| *k).collect();
    assert!(
        iter_keys.contains(&&name_a),
        "iter() must include (name_a, _)"
    );
    assert!(
        iter_keys.contains(&&name_b),
        "iter() must include (name_b, _)"
    );
    assert!(
        iter_keys.contains(&&name_c),
        "iter() must include (name_c, _)"
    );

    assert_eq!(
        (&registry).into_iter().count(),
        3,
        "IntoIterator for &WeaponRegistry must yield 3 pairs"
    );
}

#[test]
fn weapon_registry_keys_and_resolves_by_name() {
    let Ok(spec) = ron::de::from_str::<WeaponSpec>(SHIPPED_STUB_PISTOL_RON) else {
        return;
    };
    let stub_pistol = WeaponName::new("stub_pistol".to_owned());
    let registry = WeaponRegistry::new([(stub_pistol.clone(), spec)]);

    assert_eq!(
        registry.len(),
        1,
        "the registry holds the one inserted weapon"
    );
    assert!(!registry.is_empty(), "a one-weapon registry is non-empty");
    assert!(
        registry.spec(&stub_pistol).is_some(),
        "a present key resolves to its spec",
    );
    assert!(
        registry
            .spec(&WeaponName::new("missing".to_owned()))
            .is_none(),
        "an absent key resolves to None (the setup-time WeaponNotFound trigger)",
    );
}
