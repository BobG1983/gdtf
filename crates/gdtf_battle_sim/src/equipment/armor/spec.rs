//! Authored six-piece armor loadout.

use bevy::reflect::TypePath;
use serde::{Deserialize, Serialize};

use super::stats::ArmorPiece;

/// Full suit: head, torso, arms, legs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
pub struct ArmorSpec {
    /// Head piece.
    pub head:      ArmorPiece,
    /// Torso piece.
    pub torso:     ArmorPiece,
    /// Left arm.
    pub left_arm:  ArmorPiece,
    /// Right arm.
    pub right_arm: ArmorPiece,
    /// Left leg.
    pub left_leg:  ArmorPiece,
    /// Right leg.
    pub right_leg: ArmorPiece,
}

impl ArmorSpec {
    /// From ordered pieces `[head, torso, left_arm, right_arm, left_leg, right_leg]`.
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

    /// Same piece on every location.
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

    /// Ordered array of pieces.
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
