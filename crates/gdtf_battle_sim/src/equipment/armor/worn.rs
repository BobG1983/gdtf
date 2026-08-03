//! Situation-source armor loadout (deserialized into six pieces).

use serde::Deserialize;

use super::stats::{ArmorPiece, BodyPart};

/// Six pieces indexed by [`BodyPart`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(from = "SourceArmorDef")]
pub struct SourceArmor {
    pieces: [ArmorPiece; 6],
}

/// Serde shape with named fields.
#[derive(Deserialize)]
pub struct SourceArmorDef {
    head: ArmorPiece,
    torso: ArmorPiece,
    left_arm: ArmorPiece,
    right_arm: ArmorPiece,
    left_leg: ArmorPiece,
    right_leg: ArmorPiece,
}

impl From<SourceArmorDef> for SourceArmor {
    fn from(def: SourceArmorDef) -> Self {
        Self::new([
            def.head,
            def.torso,
            def.left_arm,
            def.right_arm,
            def.left_leg,
            def.right_leg,
        ])
    }
}

impl SourceArmor {
    /// From ordered pieces.
    #[must_use]
    pub const fn new(pieces: [ArmorPiece; 6]) -> Self {
        Self { pieces }
    }

    /// Same piece everywhere.
    #[must_use]
    pub const fn uniform(piece: ArmorPiece) -> Self {
        Self { pieces: [piece; 6] }
    }

    /// Piece at a body part.
    #[must_use]
    pub const fn at(&self, part: BodyPart) -> ArmorPiece {
        self.pieces[part.index()]
    }
}
