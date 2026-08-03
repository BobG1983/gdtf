//! The **authoring spec** — the `ArmorSpec` an `assets/content/armor/*.armor.ron`
use bevy::reflect::TypePath;
use serde::{Deserialize, Serialize};

use super::stats::ArmorPiece;

/// The **authoring struct** an `assets/content/armor/*.ron` deserializes into — the six
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
pub struct ArmorSpec {
        pub head:      ArmorPiece,
        pub torso:     ArmorPiece,
        pub left_arm:  ArmorPiece,
        pub right_arm: ArmorPiece,
        pub left_leg:  ArmorPiece,
        pub right_leg: ArmorPiece,
}

impl ArmorSpec {
                        #[must_use]
    pub const fn new(pieces: [ArmorPiece; 6]) -> Self {
        let [head, torso, left_arm, right_arm, left_leg, right_leg] = pieces;
        Self {
            head,
            torso,
            left_arm,
            right_arm,
            left_leg,
            right_leg,
        }
    }

                #[must_use]
    pub const fn uniform(piece: ArmorPiece) -> Self {
        Self {
            head:      piece,
            torso:     piece,
            left_arm:  piece,
            right_arm: piece,
            left_leg:  piece,
            right_leg: piece,
        }
    }

                        #[must_use]
    pub const fn pieces(&self) -> [ArmorPiece; 6] {
        [
            self.head,
            self.torso,
            self.left_arm,
            self.right_arm,
            self.left_leg,
            self.right_leg,
        ]
    }
}
