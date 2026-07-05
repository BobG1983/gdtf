//! The `ArmorType` wheel vocabulary + the dual-vocabulary `DamageType` parity
//! (matchup.md AC2/AC3).

use crate::armor::{
    ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, ArmorSpec, ArmorType,
    BodyPart,
};

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
