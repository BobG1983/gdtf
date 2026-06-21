//! The roster armor shape: the read-only [`SourceArmor`] roster record (and its
//! authoring shape [`SourceArmorDef`]).
//!
//! The mutable battle-local copy used to live here as a `WornArmor` component on the
//! ganger; since GTW-323 (ADR-0004,
//! `docs/decisions/0004-equipment-as-entities-relationships.md`) the battle-local armor
//! is modelled as related **piece entities** ([`Wears`](super::Wears)), each carrying
//! its own [`ArmorIntegrity`](super::ArmorIntegrity) component that mid-battle wear
//! degrades — so no `WornArmor` component is stored on the ganger.

use serde::Deserialize;

use super::stats::{ArmorPiece, BodyPart};

/// The **read-only** roster armor record — the persistent representation of a
/// ganger's worn armor across all six body locations (`armor_at` keyed by
/// [`BodyPart`](super::BodyPart), `weapons-and-armor.md` §"Per-hit resolution").
///
/// This is a roster source-of-truth shape: the per-location [`ArmorPiece`]s a
/// ganger carries into a battle. Per the battle-local / roster separation (ADR-0001,
/// `docs/decisions/0001-rust-bevy-rewrite.md`), this record is **never** mutated
/// during a battle. It is plain data (not a [`Component`](bevy::prelude::Component));
/// since GTW-323 (ADR-0004) the sim's battle-local armor lives on related per-piece
/// entities seeded from a registry-resolved [`ArmorSpec`](super::ArmorSpec) at setup,
/// not from this record — `SourceArmor` remains the roster authoring shape and the test
/// fixtures' uniform suit builder.
///
/// Deserializes through a per-body-part authoring shape ([`SourceArmorDef`]) that
/// routes the six named pieces through [`SourceArmor::new`] — so an authored
/// situation names each location (`head`, `torso`, …) on its own line rather than
/// writing a bare positional array, keeping the inner `pieces` array private and
/// the per-line-comment authoring convention readable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(from = "SourceArmorDef")]
pub struct SourceArmor {
    /// The six per-location armor pieces, indexed by [`BodyPart::index`](super::BodyPart::index).
    pieces: [ArmorPiece; 6],
}

/// The authored RON shape a [`SourceArmor`] deserializes from — the six
/// per-location [`ArmorPiece`]s as named fields, routed through
/// [`SourceArmor::new`] in [`BodyPart::ALL`] order.
///
/// A serde intermediate (`#[serde(from = "SourceArmorDef")]` on [`SourceArmor`]) so
/// an authored situation writes each body location's armor on its own
/// self-describing, comment-annotated line (`head: (..)`, `torso: (..)`, …) rather
/// than a bare positional 6-array — and the value still flows through the typed
/// constructor, keeping the inner `pieces` array private (no-bare-types).
#[derive(Deserialize)]
pub struct SourceArmorDef {
    /// The Head piece.
    head:      ArmorPiece,
    /// The Torso piece.
    torso:     ArmorPiece,
    /// The Left-Arm piece.
    left_arm:  ArmorPiece,
    /// The Right-Arm piece.
    right_arm: ArmorPiece,
    /// The Left-Leg piece.
    left_leg:  ArmorPiece,
    /// The Right-Leg piece.
    right_leg: ArmorPiece,
}

impl From<SourceArmorDef> for SourceArmor {
    fn from(def: SourceArmorDef) -> Self {
        // BodyPart::ALL order: Head, Torso, L-Arm, R-Arm, L-Leg, R-Leg.
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
    /// Build a roster armor record from the six per-location pieces, in
    /// [`BodyPart::ALL`] order (Head, Torso, L-Arm, R-Arm, L-Leg, R-Leg).
    #[must_use]
    pub const fn new(pieces: [ArmorPiece; 6]) -> Self {
        Self { pieces }
    }

    /// Build a roster armor record with the **same** [`ArmorPiece`] on every body
    /// location — a uniform suit. A convenience for callers (and tests) that do
    /// not yet vary armor per location.
    #[must_use]
    pub const fn uniform(piece: ArmorPiece) -> Self {
        Self { pieces: [piece; 6] }
    }

    /// The read-only armor piece protecting `part` on the roster sheet.
    #[must_use]
    pub const fn at(&self, part: BodyPart) -> ArmorPiece {
        self.pieces[part.index()]
    }
}
