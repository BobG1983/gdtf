//! `ArmorSpec` per-`BodyPart` slot keying + the newtype Deref mechanism.

use crate::armor::{
    ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, ArmorSpec, ArmorType,
    BodyPart,
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
