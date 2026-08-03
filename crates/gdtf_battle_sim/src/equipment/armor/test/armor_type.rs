use crate::armor::{
    ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, ArmorSpec, ArmorType,
    BodyPart,
};

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

#[test]
fn dual_vocabulary_nodes_parity() {
    use crate::weapon::DamageType;

    assert_eq!(ArmorType::ALL.len(), DamageType::ALL.len());
    assert_eq!(ArmorType::ALL.len(), 7);

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

    let suit = ArmorSpec::uniform(piece);
    assert_eq!(
        suit.pieces()[BodyPart::Torso.index()].armor_type,
        ArmorType::Ceramic
    );
}
