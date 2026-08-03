use crate::armor::{
    ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, ArmorSpec, ArmorType,
    BodyPart,
};

fn spec_at(spec: &ArmorSpec, part: BodyPart) -> ArmorPiece {
    spec.pieces()[part.index()]
}

fn arbitrary_source() -> ArmorSpec {
    let pieces = [
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
    ];
    ArmorSpec::new(pieces)
}

#[test]
fn spec_reads_each_piece_back_field_by_field() {
    let source = arbitrary_source();

    for part in BodyPart::ALL {
        let src = spec_at(&source, part);
        assert_eq!(*src.floor, *src.floor, "floor reads at {part:?}");
        assert_eq!(
            src,
            source.pieces()[part.index()],
            "piece at {part:?} reads back off the spec in BodyPart::ALL order",
        );
    }
    assert_ne!(
        spec_at(&source, BodyPart::Head),
        spec_at(&source, BodyPart::Torso),
        "distinct authored pieces stay in distinct slots",
    );
}

#[test]
fn body_parts_index_distinctly() {
    let mut indices: Vec<usize> = BodyPart::ALL.iter().map(|p| p.index()).collect();
    indices.sort_unstable();
    assert_eq!(indices, vec![0, 1, 2, 3, 4, 5]);
}

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
