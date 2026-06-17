//! Armor model proofs: the seed-copy isolation, the wear/`protects` gate, the
//! body-part keying, the newtype Deref, and the dual-vocabulary wheel parity.

use crate::armor::{
    ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, ArmorType, BodyPart,
    SourceArmor, WornArmor,
};

/// A roster source built from arbitrary, per-location-DISTINCT magnitudes —
/// NOT shipped tuning values. Distinct per part proves the seeding copies each
/// slot independently (not a single value smeared across all six), and using
/// arbitrary numbers keeps the test asserting the copy/isolation *mechanism*,
/// never a magnitude (magnitudes are TBD tuning).
fn arbitrary_source() -> SourceArmor {
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
    SourceArmor::new(pieces)
}

/// C7(a): a worn copy seeded from a source starts EQUAL to the source,
/// field-by-field, on every body location. Walks all six parts and compares
/// each of the four armor fields — proving `seed_from` copies the whole record
/// faithfully (the mechanism), independent of the magnitudes used.
#[test]
fn seeded_copy_equals_source_field_by_field() {
    let source = arbitrary_source();
    let worn = WornArmor::seed_from(&source);

    for part in BodyPart::ALL {
        let src = source.at(part);
        let cpy = worn.at(part);
        assert_eq!(cpy.floor, src.floor, "floor mismatch at {part:?}");
        assert_eq!(
            cpy.protection, src.protection,
            "protection mismatch at {part:?}"
        );
        assert_eq!(
            cpy.integrity, src.integrity,
            "integrity mismatch at {part:?}"
        );
        assert_eq!(cpy.hardness, src.hardness, "hardness mismatch at {part:?}");
        assert_eq!(
            cpy.armor_type, src.armor_type,
            "armor_type mismatch at {part:?}"
        );
        // And the whole piece is equal.
        assert_eq!(cpy, src, "piece mismatch at {part:?}");
    }
}

/// C7(b): mutating `integrity` on the worn copy leaves the source record
/// UNCHANGED — the battle-local/roster isolation guarantee. Wears the torso's
/// integrity on the copy, then asserts the source torso integrity is still its
/// original value, and every OTHER worn slot is untouched too.
#[test]
fn wearing_worn_integrity_does_not_mutate_source() {
    let source = arbitrary_source();
    let mut worn = WornArmor::seed_from(&source);

    let original = source.at(BodyPart::Torso).integrity;
    // Arbitrary wear amount — the mechanism, not a tuned magnitude.
    worn.wear_integrity(BodyPart::Torso, ArmorIntegrity::new(5));

    // The source torso integrity is unchanged — the roster never wears.
    assert_eq!(
        source.at(BodyPart::Torso).integrity,
        original,
        "roster source integrity must NOT change when the worn copy wears",
    );
    // The worn torso integrity DID change (the wear actually applied).
    assert_eq!(
        worn.at(BodyPart::Torso).integrity,
        ArmorIntegrity::new(*original - 5),
        "worn copy integrity must reflect the applied wear",
    );
    // Every other worn slot is untouched — wear is per-location.
    for part in BodyPart::ALL {
        if part == BodyPart::Torso {
            continue;
        }
        assert_eq!(
            worn.at(part),
            source.at(part),
            "non-struck worn slot {part:?} must equal the source",
        );
    }
}

/// Integrity may wear to AT OR BELOW zero — the "useless at `≤ 0`" gate
/// (`weapons-and-armor.md` step 3) needs a signed inner type. Wears past the
/// starting value and asserts the result is negative (not clamped, not
/// underflow-panicked), confirming the `i32` choice carries the contract.
#[test]
fn worn_integrity_can_fall_below_zero() {
    let source = SourceArmor::uniform(ArmorPiece::new(
        ArmorFloor::new(0),
        ArmorProtection::new(0),
        ArmorIntegrity::new(3),
        ArmorHardness::new(0),
        ArmorType::DEFAULT,
    ));
    let mut worn = WornArmor::seed_from(&source);

    worn.wear_integrity(BodyPart::Head, ArmorIntegrity::new(10));

    assert!(
        *worn.at(BodyPart::Head).integrity < 0,
        "worn integrity must be able to fall below zero (useless-at-≤0 gate)",
    );
    // The source is, again, untouched.
    assert_eq!(source.at(BodyPart::Head).integrity, ArmorIntegrity::new(3));
}

/// [`WornArmor::protects`] tracks the "useless at `≤ 0`" gate: a piece with
/// positive integrity protects, and once worn to `≤ 0` it stops protecting
/// (later hits resolve as bare flesh — `weapons-and-armor.md` step 3). Wears a
/// piece from positive, through exactly zero, to negative and asserts the
/// predicate flips at the crossing and stays false thereafter.
#[test]
fn protects_is_false_at_or_below_zero() {
    let source = SourceArmor::uniform(ArmorPiece::new(
        ArmorFloor::new(0),
        ArmorProtection::new(0),
        ArmorIntegrity::new(2),
        ArmorHardness::new(0),
        ArmorType::DEFAULT,
    ));
    let mut worn = WornArmor::seed_from(&source);

    // Positive integrity ⇒ still protecting.
    assert!(
        worn.protects(BodyPart::Torso),
        "a piece with integrity > 0 must protect",
    );

    // Wear exactly TO zero — the boundary itself is unprotected (≤ 0, not < 0).
    worn.wear_integrity(BodyPart::Torso, ArmorIntegrity::new(2));
    assert_eq!(*worn.at(BodyPart::Torso).integrity, 0);
    assert!(
        !worn.protects(BodyPart::Torso),
        "a piece worn to exactly 0 must stop protecting (≤ 0 gate)",
    );

    // Wear further below zero — still unprotected.
    worn.wear_integrity(BodyPart::Torso, ArmorIntegrity::new(5));
    assert!(
        *worn.at(BodyPart::Torso).integrity < 0,
        "integrity must fall below zero",
    );
    assert!(
        !worn.protects(BodyPart::Torso),
        "a piece worn below zero must stay unprotected (bare flesh)",
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

    // And it survives the seed copy onto a worn piece (the field is part of the
    // copied record, like the four stats).
    let worn = WornArmor::seed_from(&SourceArmor::uniform(piece));
    assert_eq!(worn.at(BodyPart::Torso).armor_type, ArmorType::Ceramic);
}
