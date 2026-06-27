//! Relocated tests for the `WeaponSpec` authoring struct + the `WeaponRegistry`
//! (GTW-201; moved VERBATIM — was the `=== GTW-257 ===` block of the flat module).

use super::support::*;

/// A shipped weapon `.ron`, read at compile time via the same `include_str!`
/// pattern `tuning.rs` / `situation.rs` use — the REAL on-disk authored file
/// (`assets/content/weapons/stub_pistol.weapon.ron`), so a regression in the authored file
/// turns this red.
const SHIPPED_STUB_PISTOL_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/content/weapons/stub_pistol.weapon.ron"
));

/// A shipped TWO-handed long-arm `.weapon.ron` — the GTW-443 `handedness:` regression
/// witness (the real on-disk `las_carbine`, authored `TwoHanded`).
const SHIPPED_LAS_CARBINE_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/content/weapons/las_carbine.weapon.ron"
));

/// GTW-257 AC1 — the shipped `assets/content/weapons/stub_pistol.weapon.ron` parses into a
/// `WeaponSpec`, and `into_bundle(name)` yields a `WeaponBundle` carrying that
/// `WeaponName` + a `FireMode` list whose modes carry their `ModeKind`.
/// Value-agnostic on the tunable cone/TU/damage magnitudes (the authored numbers
/// are DATA, not pinned by the test): it asserts the NAME landed and the
/// fire-mode offers at least one mode whose `single()` is `Single`-kind, never a
/// magnitude.
#[test]
fn shipped_weapon_spec_parses_and_converts_to_a_bundle() {
    let parsed = ron::de::from_str::<WeaponSpec>(SHIPPED_STUB_PISTOL_RON);
    assert!(
        parsed.is_ok(),
        "the shipped assets/content/weapons/stub_pistol.weapon.ron must parse into a WeaponSpec: {parsed:?}",
    );
    let Ok(spec) = parsed else {
        return;
    };

    // The file does NOT author a name — the name is the FILE KEY, supplied here
    // (the loader supplies the filename stem). into_bundle carries it through.
    let key = WeaponName::new("stub_pistol".to_owned());
    let bundle = spec.into_bundle(key);
    assert_eq!(
        &*bundle.name, "stub_pistol",
        "into_bundle must carry the supplied WeaponName (the file key) onto the bundle",
    );
    // The Weapon marker is added by into_bundle (it is NOT authored in the file).
    assert_eq!(bundle.marker, Weapon, "into_bundle adds the Weapon marker");
    // The selector offers at least one mode, and single() is the Single-kind one
    // (mechanism, not a value) — the authored list round-tripped.
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

/// GTW-443 C1 — a weapon entity carries its authored [`Handedness`] from the
/// `handedness:` field of its `.weapon.ron`, parsed by the existing loader path. The
/// shipped `TwoHanded` `las_carbine` resolves to [`Handedness::TwoHanded`] and the
/// `OneHanded` `stub_pistol` to [`Handedness::OneHanded`] on the spawned bundle. Enum
/// resolves only (no magnitude pin, per the loader-tests rule) — a swap of the authored
/// variant would flip this.
#[test]
fn shipped_weapon_handedness_parses_onto_the_bundle() {
    // A TwoHanded long-arm → TwoHanded on the bundle.
    let carbine = ron::de::from_str::<WeaponSpec>(SHIPPED_LAS_CARBINE_RON);
    assert!(
        carbine.is_ok(),
        "the shipped las_carbine.weapon.ron must parse into a WeaponSpec: {carbine:?}",
    );
    let Ok(carbine) = carbine else { return };
    let carbine_bundle = carbine.into_bundle(WeaponName::new("las_carbine".to_owned()));
    assert_eq!(
        carbine_bundle.handedness,
        Handedness::TwoHanded,
        "a shipped two-handed long-arm parses to Handedness::TwoHanded",
    );

    // A OneHanded pistol → OneHanded on the bundle.
    let pistol = ron::de::from_str::<WeaponSpec>(SHIPPED_STUB_PISTOL_RON);
    assert!(
        pistol.is_ok(),
        "the shipped stub_pistol.weapon.ron must parse into a WeaponSpec: {pistol:?}",
    );
    let Ok(pistol) = pistol else { return };
    let pistol_bundle = pistol.into_bundle(WeaponName::new("stub_pistol".to_owned()));
    assert_eq!(
        pistol_bundle.handedness,
        Handedness::OneHanded,
        "a shipped one-handed pistol parses to Handedness::OneHanded",
    );
}

/// GTW-257 AC1 — a `WeaponSpec` round-trips from inline RON (no shipped magnitudes)
/// and `into_bundle` groups the damage / handling blocks faithfully: a spot value
/// read back off the bundle equals the authored one. Arbitrary literals
/// (mechanism, not a balance pin), proving the authoring shape and the conversion.
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

    let bundle = spec.into_bundle(WeaponName::new("test-gun".to_owned()));
    // Spot values flowed through the DamageProfile / HandlingProfile grouping
    // (distinct arbitrary literals so a field swap would surface).
    assert_eq!(*bundle.damage, 14i32, "damage flows through DamageProfile");
    assert_eq!(*bundle.punch, 6i32, "punch flows through DamageProfile");
    assert_eq!(*bundle.shred, 4i32, "shred flows through DamageProfile");
    assert_eq!(bundle.damage_type, DamageType::Kinetic);
    // GTW-275: the authored `magazine: (size, reload_tu)` grouping parses, and
    // into_bundle spawns the magazine FULL (loaded == size). The reload_tu magnitude is
    // tunable (not pinned); only the size + full-load RELATION is asserted.
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
    // The two authored modes survived the parse + grouping, in order.
    let kinds: Vec<ModeKind> = bundle.fire_mode.iter().map(|spec| spec.kind).collect();
    assert_eq!(kinds, vec![ModeKind::Single, ModeKind::Burst]);
}

/// GTW-413 AC2 — `WeaponRegistry::keys` and `WeaponRegistry::iter` enumerate
/// exactly the keys and (key, spec) pairs that were inserted. Three distinct
/// [`WeaponName`]s built from arbitrary RON (mechanism, not pinned magnitudes):
/// asserts count == 3 AND each expected key is present via `keys()`; asserts
/// `iter()` yields the same 3 key-spec pairs by key membership (value-agnostic).
/// `HashMap` order is unspecified — membership is the assertion, not position.
#[test]
fn weapon_registry_keys_and_iter_enumerate_all_entries() {
    // Three minimal WeaponSpecs from the same inline RON (arbitrary, not shipped
    // magnitudes) — we only need distinct WeaponName keys; the spec values are
    // incidental (value-agnostic per the loader-tests rule).
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

    // keys() — count + membership (order unspecified).
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

    // iter() — same count, key membership (value-agnostic on the spec side).
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

    // IntoIterator for &WeaponRegistry must yield the same count (trait impl proof).
    assert_eq!(
        (&registry).into_iter().count(),
        3,
        "IntoIterator for &WeaponRegistry must yield 3 pairs"
    );
}

/// GTW-257 — a `WeaponRegistry` keys specs by `WeaponName` and resolves a lookup:
/// a present key returns the spec, an absent key returns `None`. Built directly
/// from `WeaponRegistry::new` (no `AssetServer` — the sim-unit shape AC2/AC3 use).
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
