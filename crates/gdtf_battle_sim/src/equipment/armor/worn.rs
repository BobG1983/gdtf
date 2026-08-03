//! authoring shape [`SourceArmorDef`]).
use serde::Deserialize;

use super::stats::{ArmorPiece, BodyPart};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(from = "SourceArmorDef")]
pub struct SourceArmor {
        pieces: [ArmorPiece; 6],
}

/// A serde intermediate (`#[serde(from = "SourceArmorDef")]` on [`SourceArmor`]) so
#[derive(Deserialize)]
pub struct SourceArmorDef {
        head:      ArmorPiece,
        torso:     ArmorPiece,
        left_arm:  ArmorPiece,
        right_arm: ArmorPiece,
        left_leg:  ArmorPiece,
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
            #[must_use]
    pub const fn new(pieces: [ArmorPiece; 6]) -> Self {
        Self { pieces }
    }

                #[must_use]
    pub const fn uniform(piece: ArmorPiece) -> Self {
        Self { pieces: [piece; 6] }
    }

        #[must_use]
    pub const fn at(&self, part: BodyPart) -> ArmorPiece {
        self.pieces[part.index()]
    }
}
