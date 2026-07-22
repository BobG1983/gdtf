//! The **authoring spec** — the `ArmorSpec` an `assets/content/armor/*.armor.ron`
//! deserializes into (GTW-269), the armor mirror of
//! [`WeaponSpec`](crate::weapon::WeaponSpec).

use bevy::reflect::TypePath;
use serde::{Deserialize, Serialize};

use super::stats::ArmorPiece;

/// The **authoring struct** an `assets/content/armor/*.ron` deserializes into — the six
/// per-location [`ArmorPiece`]s a suit carries, as named fields, MINUS the
/// [`ArmorName`](super::ArmorName) (the name is the FILE KEY, supplied by the
/// loader from the file's stem).
///
/// This is the data-driven, folder-loaded armor model (the
/// [[weapons-armor-data-driven]] end-state, GTW-269): a per-armor loose `.ron`
/// file is parsed into an `ArmorSpec`, keyed by its filename stem into the
/// [`ArmorRegistry`](super::ArmorRegistry), and resolved at battle setup into the
/// roster [`SourceArmor`](super::SourceArmor) record (slice C). It mirrors the
/// [`SourceArmorDef`](super::worn::SourceArmorDef) authoring shape exactly — the
/// same six named body-location fields, in [`BodyPart::ALL`](super::BodyPart::ALL)
/// order — dropping only the name the loader owns (the file key).
///
/// Every field is an [`ArmorPiece`] authored as its named-field RON record (the
/// per-line-comment authoring convention); the authored magnitudes are tuning DATA
/// (commented in the `.ron`), NOT pinned by tests (the brittle-test rule). Derives
/// [`Deserialize`] so the loose `.ron` parses, and [`TypePath`] because the
/// `RonAsset<ArmorSpec>` the loader wraps it in requires its payload to be
/// [`TypePath`] (the same bound
/// [`WeaponSpec`](crate::equipment::weapon::WeaponSpec) /
/// [`Situation`](crate::lifecycle::situation::Situation) satisfy).
///
/// **`Copy`** — an [`ArmorPiece`] is `Copy` (it owns only `i32`/enum leaves), so an
/// authored suit of six is `Copy` too, mirroring [`SourceArmor`](super::SourceArmor).
///
/// Derives [`Serialize`] too (GTW-479): the content editor's ARMOR authoring mode
/// WRITES an edited spec back to a `.armor.ron` through the shared RON save path
/// (the [`GangRoster`](crate::ganger::GangRoster) / `TerrainDef` write precedent),
/// so the authoring struct must serialise to exactly the shape it deserialises from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
pub struct ArmorSpec {
    /// The Head piece — protects the rarely-struck, severity-amplifying head.
    pub head:      ArmorPiece,
    /// The Torso piece — protects the bulk of the silhouette, where most hits land.
    pub torso:     ArmorPiece,
    /// The Left-Arm piece.
    pub left_arm:  ArmorPiece,
    /// The Right-Arm piece.
    pub right_arm: ArmorPiece,
    /// The Left-Leg piece.
    pub left_leg:  ArmorPiece,
    /// The Right-Leg piece.
    pub right_leg: ArmorPiece,
}

impl ArmorSpec {
    /// Build an armor spec from the six per-location pieces, in
    /// [`BodyPart::ALL`](super::BodyPart::ALL) order (Head, Torso, L-Arm, R-Arm,
    /// L-Leg, R-Leg) — the array shape [`pieces`](ArmorSpec::pieces) returns, so a
    /// caller (and the test fixtures) can construct a spec from a positional six
    /// without naming each field.
    #[must_use]
    pub const fn new(pieces: [ArmorPiece; 6]) -> Self {
        // BodyPart::ALL order: Head, Torso, L-Arm, R-Arm, L-Leg, R-Leg.
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

    /// Build an armor spec with the **same** [`ArmorPiece`] on every body location —
    /// a uniform suit. A convenience for callers (and tests) that do not vary armor
    /// per location, mirroring [`SourceArmor::uniform`](super::SourceArmor::uniform).
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

    /// The six authored pieces in [`BodyPart::ALL`](super::BodyPart::ALL) order
    /// (Head, Torso, L-Arm, R-Arm, L-Leg, R-Leg) — the array shape the roster
    /// [`SourceArmor::new`](super::SourceArmor::new) keys by, mirroring
    /// [`SourceArmorDef`](super::worn::SourceArmorDef)'s `From` ordering. `i` is
    /// [`BodyPart::ALL`](super::BodyPart::ALL)`[i]`.
    #[must_use]
    pub const fn pieces(&self) -> [ArmorPiece; 6] {
        // BodyPart::ALL order: Head, Torso, L-Arm, R-Arm, L-Leg, R-Leg.
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
