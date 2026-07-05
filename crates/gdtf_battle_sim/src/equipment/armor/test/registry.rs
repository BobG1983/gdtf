//! `ArmorRegistry` keying/enumeration + the RON round-trip resolve (GTW-269 /
//! GTW-413).

use crate::armor::{
    ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorName, ArmorPiece, ArmorProtection,
    ArmorRegistry, ArmorSpec, ArmorType, BodyPart,
};

/// The six pieces the inline-authored `ArmorSpec` RON below is EXPECTED to parse
/// into, in [`BodyPart::ALL`] order — per-location-DISTINCT arbitrary magnitudes
/// and a distinct [`ArmorType`] per slot, so a field that lands in the wrong slot
/// surfaces. The mechanism, not pinned tuning. Mirrors the RON literal field for
/// field.
fn expected_round_trip_pieces() -> [ArmorPiece; 6] {
    [
        ArmorPiece::new(
            ArmorFloor::new(1),
            ArmorProtection::new(11),
            ArmorIntegrity::new(21),
            ArmorHardness::new(31),
            ArmorType::Plated,
        ),
        ArmorPiece::new(
            ArmorFloor::new(2),
            ArmorProtection::new(12),
            ArmorIntegrity::new(22),
            ArmorHardness::new(32),
            ArmorType::Refractive,
        ),
        ArmorPiece::new(
            ArmorFloor::new(3),
            ArmorProtection::new(13),
            ArmorIntegrity::new(23),
            ArmorHardness::new(33),
            ArmorType::Flak,
        ),
        ArmorPiece::new(
            ArmorFloor::new(4),
            ArmorProtection::new(14),
            ArmorIntegrity::new(24),
            ArmorHardness::new(34),
            ArmorType::Void,
        ),
        ArmorPiece::new(
            ArmorFloor::new(5),
            ArmorProtection::new(15),
            ArmorIntegrity::new(25),
            ArmorHardness::new(35),
            ArmorType::Hazard,
        ),
        ArmorPiece::new(
            ArmorFloor::new(6),
            ArmorProtection::new(16),
            ArmorIntegrity::new(26),
            ArmorHardness::new(36),
            ArmorType::Reinforced,
        ),
    ]
}

/// GTW-413 AC2 — `ArmorRegistry::keys` and `ArmorRegistry::iter` enumerate
/// exactly the keys and (key, spec) pairs that were inserted. Three distinct
/// [`ArmorName`]s with an arbitrary uniform [`ArmorSpec`] (mechanism, not pinned
/// magnitudes): asserts count == 3 AND each expected key is present via
/// `keys()`; asserts `iter()` yields the same 3 key-spec pairs by key
/// membership (value-agnostic on the spec side). `HashMap` order is unspecified
/// — membership is the assertion, not position.
#[test]
fn armor_registry_keys_and_iter_enumerate_all_entries() {
    // A uniform ArmorSpec built from arbitrary magnitudes (not shipped tuning
    // values) — we only need distinct ArmorName keys; the spec value is incidental.
    let piece = ArmorPiece::new(
        ArmorFloor::new(1),
        ArmorProtection::new(2),
        ArmorIntegrity::new(3),
        ArmorHardness::new(4),
        crate::armor::ArmorType::Plated,
    );
    let spec = ArmorSpec::uniform(piece);

    let name_a = ArmorName::new("aegis".to_owned());
    let name_b = ArmorName::new("bastion".to_owned());
    let name_c = ArmorName::new("carapace".to_owned());

    let registry = ArmorRegistry::new([
        (name_a.clone(), spec),
        (name_b.clone(), spec),
        (name_c.clone(), spec),
    ]);

    // keys() — count + membership (order unspecified).
    let all_keys: Vec<&ArmorName> = registry.keys().collect();
    assert_eq!(all_keys.len(), 3, "keys() must yield exactly 3 entries");
    assert!(
        all_keys.contains(&&name_a),
        "keys() must include name_a (\"aegis\")"
    );
    assert!(
        all_keys.contains(&&name_b),
        "keys() must include name_b (\"bastion\")"
    );
    assert!(
        all_keys.contains(&&name_c),
        "keys() must include name_c (\"carapace\")"
    );

    // iter() — same count, key membership (value-agnostic on the spec side).
    let all_pairs: Vec<(&ArmorName, &ArmorSpec)> = registry.iter().collect();
    assert_eq!(
        all_pairs.len(),
        3,
        "iter() must yield exactly 3 (key, spec) pairs"
    );
    let iter_keys: Vec<&ArmorName> = all_pairs.iter().map(|(k, _)| *k).collect();
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

    // IntoIterator for &ArmorRegistry must yield the same count (trait impl proof).
    assert_eq!(
        (&registry).into_iter().count(),
        3,
        "IntoIterator for &ArmorRegistry must yield 3 pairs"
    );
}

/// GTW-269 (the ticket-required round-trip, mirroring
/// `weapon_spec_round_trips_and_into_bundle_groups_faithfully`): an authored
/// 6-piece [`ArmorSpec`] parses from inline RON, an [`ArmorRegistry`] keys it by
/// [`ArmorName`], `spec()` resolves it back, and every one of the five fields
/// (floor / protection / integrity / hardness / `armor_type`) of each of the six
/// pieces matches the authored values — read off [`ArmorSpec::pieces`] in
/// [`BodyPart::ALL`] order. Per-location-DISTINCT arbitrary magnitudes (the
/// mechanism, not pinned tuning) prove each named field round-trips into the right
/// slot.
#[test]
fn armor_spec_round_trips_and_registry_resolves_by_name() {
    // Authored six-piece suit — each field a self-describing line (the per-line
    // authoring convention). Distinct floor/protection/integrity/hardness per slot
    // and a distinct ArmorType per slot prove each named field maps to its slot.
    let authored = r"(
        head:      ( floor: 1, protection: 11, integrity: 21, hardness: 31, armor_type: Plated ),
        torso:     ( floor: 2, protection: 12, integrity: 22, hardness: 32, armor_type: Refractive ),
        left_arm:  ( floor: 3, protection: 13, integrity: 23, hardness: 33, armor_type: Flak ),
        right_arm: ( floor: 4, protection: 14, integrity: 24, hardness: 34, armor_type: Void ),
        left_leg:  ( floor: 5, protection: 15, integrity: 25, hardness: 35, armor_type: Hazard ),
        right_leg: ( floor: 6, protection: 16, integrity: 26, hardness: 36, armor_type: Reinforced ),
    )";
    let parsed = ron::de::from_str::<ArmorSpec>(authored);
    assert!(
        parsed.is_ok(),
        "inline ArmorSpec RON must parse: {parsed:?}"
    );
    let Ok(spec) = parsed else {
        return;
    };

    // Key it into a registry by name and resolve it back (the lookup mechanism).
    let armor_name = ArmorName::new("flak-jacket".to_owned());
    let registry = ArmorRegistry::new([(armor_name.clone(), spec)]);
    assert_eq!(
        registry.len(),
        1,
        "the registry holds the one inserted armor"
    );
    assert!(!registry.is_empty(), "a one-armor registry is non-empty");
    assert!(
        registry
            .spec(&ArmorName::new("missing".to_owned()))
            .is_none(),
        "an absent key resolves to None",
    );
    let resolved = registry.spec(&armor_name);
    assert!(
        resolved.is_some(),
        "the present key must resolve to its spec",
    );
    let Some(resolved) = resolved else {
        return;
    };

    // The authored pieces, in BodyPart::ALL order, with their five expected fields.
    let expected = expected_round_trip_pieces();

    // Walk all six parts; assert each of the five fields per piece matches the
    // authored value (pieces() returns them in BodyPart::ALL order).
    let pieces = resolved.pieces();
    for (i, part) in BodyPart::ALL.into_iter().enumerate() {
        let got = pieces[i];
        let want = expected[i];
        assert_eq!(got.floor, want.floor, "floor mismatch at {part:?}");
        assert_eq!(
            got.protection, want.protection,
            "protection mismatch at {part:?}"
        );
        assert_eq!(
            got.integrity, want.integrity,
            "integrity mismatch at {part:?}"
        );
        assert_eq!(got.hardness, want.hardness, "hardness mismatch at {part:?}");
        assert_eq!(
            got.armor_type, want.armor_type,
            "armor_type mismatch at {part:?}"
        );
        // And the whole piece round-tripped.
        assert_eq!(got, want, "piece mismatch at {part:?}");
    }
}
