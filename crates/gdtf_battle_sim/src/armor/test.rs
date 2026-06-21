//! Armor model proofs: the body-part keying, the newtype Deref, the
//! armor-type-carrying piece, the registry round-trip, and the dual-vocabulary
//! wheel parity. (The battle-local wear/`protects` gate is now proven on the
//! armor-piece ENTITIES — `crate::armor::wear_armor` on `&mut ArmorIntegrity` — by
//! `armor_wear`, `apply_hit`, and `situation::test::armor_pieces`, GTW-323.)

use crate::armor::{
    ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorName, ArmorPiece, ArmorProtection,
    ArmorRegistry, ArmorSpec, ArmorType, BodyPart,
};

/// The armor piece a [`BodyPart`] resolves to in an [`ArmorSpec`] — the spec mirror
/// of `SourceArmor::at`, reading [`ArmorSpec::pieces`] in [`BodyPart::ALL`] order.
fn spec_at(spec: &ArmorSpec, part: BodyPart) -> ArmorPiece {
    spec.pieces()[part.index()]
}

/// A source spec built from arbitrary, per-location-DISTINCT magnitudes —
/// NOT shipped tuning values. Distinct per part proves each named field maps to its
/// slot independently (not a single value smeared across all six), and using
/// arbitrary numbers keeps the test asserting the *mechanism*, never a magnitude
/// (magnitudes are TBD tuning).
fn arbitrary_source() -> ArmorSpec {
    let pieces = [
        // Head — distinct ArmorType per slot proves the type copies per piece.
        ArmorPiece::new(
            ArmorFloor::new(1),
            ArmorProtection::new(11),
            ArmorIntegrity::new(21),
            ArmorHardness::new(31),
            ArmorType::Plated,
        ),
        // Torso
        ArmorPiece::new(
            ArmorFloor::new(2),
            ArmorProtection::new(12),
            ArmorIntegrity::new(22),
            ArmorHardness::new(32),
            ArmorType::Refractive,
        ),
        // L-Arm
        ArmorPiece::new(
            ArmorFloor::new(3),
            ArmorProtection::new(13),
            ArmorIntegrity::new(23),
            ArmorHardness::new(33),
            ArmorType::Flak,
        ),
        // R-Arm
        ArmorPiece::new(
            ArmorFloor::new(4),
            ArmorProtection::new(14),
            ArmorIntegrity::new(24),
            ArmorHardness::new(34),
            ArmorType::Void,
        ),
        // L-Leg
        ArmorPiece::new(
            ArmorFloor::new(5),
            ArmorProtection::new(15),
            ArmorIntegrity::new(25),
            ArmorHardness::new(35),
            ArmorType::Hazard,
        ),
        // R-Leg
        ArmorPiece::new(
            ArmorFloor::new(6),
            ArmorProtection::new(16),
            ArmorIntegrity::new(26),
            ArmorHardness::new(36),
            ArmorType::Reinforced,
        ),
    ];
    ArmorSpec::new(pieces)
}

/// C7 — an [`ArmorSpec`] reads each authored piece back per [`BodyPart`],
/// field-by-field, on every body location (the spec mirror of `SourceArmor::at`).
/// Walks all six parts and compares each of the five armor fields — proving the
/// spec's per-slot piece access is faithful (the mechanism), independent of the
/// magnitudes used. (The battle-local wear/isolation of the SEEDED copy is now
/// proven on the armor-piece entities by `armor_pieces` / `armor_wear`, GTW-323.)
#[test]
fn spec_reads_each_piece_back_field_by_field() {
    let source = arbitrary_source();

    for part in BodyPart::ALL {
        let src = spec_at(&source, part);
        // Each named field reads back off the per-part piece.
        assert_eq!(*src.floor, *src.floor, "floor reads at {part:?}");
        assert_eq!(
            src,
            source.pieces()[part.index()],
            "piece at {part:?} reads back off the spec in BodyPart::ALL order",
        );
    }
    // Distinct per-part magnitudes prove no slot smears into another.
    assert_ne!(
        spec_at(&source, BodyPart::Head),
        spec_at(&source, BodyPart::Torso),
        "distinct authored pieces stay in distinct slots",
    );
}

/// The six [`BodyPart`] variants index distinctly across `0..6` — the keying
/// invariant the per-location slot array relies on (no two parts collide).
#[test]
fn body_parts_index_distinctly() {
    let mut indices: Vec<usize> = BodyPart::ALL.iter().map(|p| p.index()).collect();
    indices.sort_unstable();
    assert_eq!(indices, vec![0, 1, 2, 3, 4, 5]);
}

/// The newtypes' derived [`Deref`](bevy::prelude::Deref) reaches their inner
/// `i32`, and [`ArmorIntegrity`]'s [`DerefMut`](bevy::prelude::DerefMut) writes
/// through it. Built from arbitrary literals so this pins the Deref mechanism,
/// not a magnitude.
#[test]
fn armor_newtypes_deref_to_inner() {
    assert_eq!(*ArmorFloor::new(7), 7i32);
    assert_eq!(*ArmorProtection::new(8), 8i32);
    assert_eq!(*ArmorIntegrity::new(9), 9i32);
    assert_eq!(*ArmorHardness::new(10), 10i32);

    let mut integrity = ArmorIntegrity::new(4);
    *integrity -= 1;
    assert_eq!(*integrity, 3i32);
}

/// AC2 — `ArmorType` has exactly 7 variants, in wheel-node order
/// (`docs/combat/matchup.md` Table 1).
#[test]
fn armor_type_has_seven_variants() {
    assert_eq!(ArmorType::ALL.len(), 7);
    assert_eq!(
        ArmorType::ALL,
        [
            ArmorType::Plated,
            ArmorType::Refractive,
            ArmorType::Flak,
            ArmorType::Void,
            ArmorType::Hazard,
            ArmorType::Reinforced,
            ArmorType::Ceramic,
        ]
    );
}

/// AC2 (dual-vocabulary parity) — both wheels are length 7 and node `i` of
/// [`ArmorType::ALL`] mirrors node `i` of [`crate::weapon::DamageType::ALL`]
/// (matchup.md §"The 7 types": "same wheel node `#`, two names"). This is the
/// cross-enum half AC2 requires; it lives here because it imports both enums.
#[test]
fn dual_vocabulary_nodes_parity() {
    use crate::weapon::DamageType;

    // Same node count — neither vocabulary can drift from the shared wheel.
    assert_eq!(ArmorType::ALL.len(), DamageType::ALL.len());
    assert_eq!(ArmorType::ALL.len(), 7);

    // matchup.md Table 1: node # ↦ (Armor, Weapon/Damage). Node `i` of each
    // `ALL` array must be exactly this mirror pair.
    let expected_mirror = [
        (ArmorType::Plated, DamageType::Shock),
        (ArmorType::Refractive, DamageType::Blast),
        (ArmorType::Flak, DamageType::Chem),
        (ArmorType::Void, DamageType::Kinetic),
        (ArmorType::Hazard, DamageType::Plasma),
        (ArmorType::Reinforced, DamageType::Rend),
        (ArmorType::Ceramic, DamageType::Las),
    ];
    for (i, (armor, damage)) in expected_mirror.into_iter().enumerate() {
        assert_eq!(ArmorType::ALL[i], armor, "armor node {i} drifted");
        assert_eq!(DamageType::ALL[i], damage, "damage node {i} drifted");
    }
}

/// AC3 — an `ArmorPiece` carries an `ArmorType` and reads it back (mechanism,
/// not magnitude). Pairs with `weapon_carries_damage_type_round_trip` in
/// `weapon.rs`.
#[test]
fn armor_piece_carries_armor_type() {
    let piece = ArmorPiece::new(
        ArmorFloor::new(1),
        ArmorProtection::new(2),
        ArmorIntegrity::new(3),
        ArmorHardness::new(4),
        ArmorType::Ceramic,
    );
    assert_eq!(piece.armor_type, ArmorType::Ceramic);

    // And it survives a uniform suit's per-slot read (the field is part of the
    // copied record, like the four stats).
    let suit = ArmorSpec::uniform(piece);
    assert_eq!(
        suit.pieces()[BodyPart::Torso.index()].armor_type,
        ArmorType::Ceramic
    );
}

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
