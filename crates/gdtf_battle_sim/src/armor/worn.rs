//! The roster / battle-local armor split: the read-only [`SourceArmor`] roster
//! record (and its authoring shape), and the mutable battle-local [`WornArmor`]
//! copy that mid-battle wear degrades.

use bevy::prelude::Component;
use serde::Deserialize;

use super::{
    spec::ArmorSpec,
    stats::{ArmorIntegrity, ArmorPiece, BodyPart},
};

/// The **read-only** roster armor record — the persistent representation of a
/// ganger's worn armor across all six body locations (`armor_at` keyed by
/// [`BodyPart`], `weapons-and-armor.md` §"Per-hit resolution").
///
/// This is a roster source-of-truth shape: the per-location [`ArmorPiece`]s a
/// ganger carries into a battle. Per the battle-local / roster separation (ADR-0001,
/// `docs/decisions/0001-rust-bevy-rewrite.md`), this record is **never** mutated
/// during a battle. It is plain data (not a [`Component`]); the battle-local
/// [`WornArmor`] copy is the component the sim places on ganger entities (since
/// GTW-269 that copy is seeded from a registry-resolved
/// [`ArmorSpec`](super::ArmorSpec) via [`WornArmor::seed_from`], not from this record
/// — `SourceArmor` remains the roster authoring shape and the test fixtures' uniform
/// suit builder).
///
/// Deserializes through a per-body-part authoring shape ([`SourceArmorDef`]) that
/// routes the six named pieces through [`SourceArmor::new`] — so an authored
/// situation names each location (`head`, `torso`, …) on its own line rather than
/// writing a bare positional array, keeping the inner `pieces` array private and
/// the per-line-comment authoring convention readable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(from = "SourceArmorDef")]
pub struct SourceArmor {
    /// The six per-location armor pieces, indexed by [`BodyPart::index`].
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

/// The **battle-local** worn-armor copy for one ganger — the sim's only mutable
/// armor surface during a battle: the battle-local **worn armor**, seeded at
/// construction, that mid-battle wear mutates (the model/view battle-local /
/// roster separation; ADR-0001, `docs/decisions/0001-rust-bevy-rewrite.md`).
///
/// A Bevy [`Component`] keyed by [`BodyPart`] across the six parts: per-hit wear
/// degrades the struck location's [`ArmorIntegrity`] in place
/// (`weapons-and-armor.md` §"Per-hit resolution" step 3), and a location worn to
/// `integrity ≤ 0` stops protecting for the rest of the battle. It is seeded **by
/// value** from a read-only [`SourceArmor`] ([`seed_from`](WornArmor::seed_from)),
/// so a worn-copy mutation can never leak back to the roster source.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WornArmor {
    /// The six per-location worn pieces, indexed by [`BodyPart::index`]. Only the
    /// [`ArmorIntegrity`] of each wears mid-battle.
    pieces: [ArmorPiece; 6],
}

impl WornArmor {
    /// Seed a battle-local worn copy **from** a resolved [`ArmorSpec`] (the suit the
    /// [`ArmorRegistry`](super::ArmorRegistry) keyed by a ganger's armor key at
    /// setup), **by value**.
    ///
    /// This is the C4 seeding function: it produces the owned battle-local copy
    /// the sim mutates during the battle. Because [`ArmorPiece`] is `Copy` and the
    /// six pieces are read by value out of the spec (in
    /// [`BodyPart::ALL`](super::BodyPart::ALL) order via
    /// [`ArmorSpec::pieces`](super::ArmorSpec::pieces)), no reference to the source
    /// record survives — a later [`wear_integrity`](WornArmor::wear_integrity) on
    /// this copy can never reach the spec the registry holds (the roster data is
    /// never touched; ADR-0001, `docs/decisions/0001-rust-bevy-rewrite.md`).
    #[must_use]
    pub const fn seed_from(source: &ArmorSpec) -> Self {
        Self {
            pieces: source.pieces(),
        }
    }

    /// The current worn armor piece protecting `part`.
    #[must_use]
    pub const fn at(&self, part: BodyPart) -> ArmorPiece {
        self.pieces[part.index()]
    }

    /// Whether the worn piece at `part` still protects — its
    /// [`ArmorIntegrity`] is **strictly above zero**
    /// (`weapons-and-armor.md` §"Per-hit resolution" step 3: "useless at `≤ 0`").
    ///
    /// A piece worn to `integrity ≤ 0` stops protecting for the rest of the
    /// battle: later hits on that location resolve as **bare flesh** (the doc's
    /// "later hits on that location resolve as bare flesh"). This is the gate the
    /// per-hit resolver reads to decide whether the struck location still soaks.
    #[must_use]
    pub fn protects(&self, part: BodyPart) -> bool {
        *self.at(part).integrity > 0
    }

    /// Degrade the worn integrity at `part` by `wear`, mutating **only** this
    /// battle-local copy (`weapons-and-armor.md` §"Per-hit resolution" step 3:
    /// `integrity −= …`).
    ///
    /// Subtracts the wear amount, letting integrity fall **at or below zero** (a
    /// piece at `≤ 0` stops protecting — the caller reads that gate). Only
    /// [`ArmorIntegrity`] is touched; hardness, protection, and floor are left
    /// intact, per the doc ("Hardness does not degrade"). `wear` is an
    /// [`ArmorIntegrity`] delta — the formula's per-hit integrity term
    /// (`min(protection, damage) + effPen + shred`) is summed by the (later)
    /// resolver and handed in as the integrity it consumes.
    pub fn wear_integrity(&mut self, part: BodyPart, wear: ArmorIntegrity) {
        let piece = &mut self.pieces[part.index()];
        piece.integrity = ArmorIntegrity::new(*piece.integrity - *wear);
    }
}
