//! Armor: the four per-location armor stats, an [`ArmorPiece`] bundling them for
//! one body location, the read-only roster [`SourceArmor`] record, and the
//! battle-local [`WornArmor`] component that mid-battle wear mutates.
//!
//! This is the E1.3 battle-local-armor slice. The armor model is shared by
//! gangers and cover (`docs/combat/resolution.md` §3) and feeds the per-hit
//! formula in `docs/combat/weapons-and-armor.md` §"Per-hit resolution":
//!
//! 1. `effPen = max(0, punch − hardness)`
//! 2. `dmg    = max(floor, damage − max(0, protection − effPen))`
//! 3. `integrity −= min(protection, damage) + effPen + shred` — useless at `≤ 0`
//!
//! The four armor fields are each a named newtype (no-bare-types) over `i32`:
//! the formula subtracts and clamps these against weapon stats, and `integrity`
//! must track **below zero** ("useless at `≤ 0`"), so a *signed* integer is the
//! honest inner type — it never underflows on the `protection − effPen` and
//! `integrity − wear` subtractions the way an unsigned type would. The magnitudes
//! are TBD tuning (`weapons-and-armor.md` §"TBD"); only the *mechanism* is built
//! and tested here. Per `weapons-and-armor.md`, **hardness does not degrade** —
//! only [`ArmorIntegrity`] wears, so it is the worn copy's single mutable field.
//!
//! The model/view split (`docs/architecture.md`): the [`SourceArmor`] record is
//! the roster representation and is **never** mutated during a battle; a battle
//! starts by seeding an owned [`WornArmor`] copy **by value** from it, and only
//! that copy wears. Seeding by value means a worn-copy mutation can never leak
//! back to the roster source. The full situation→entities setup that places these
//! components on ganger entities is E1.8 (GTW-158), not this slice.

use bevy::prelude::{Component, Deref, DerefMut};

/// The minimum damage a landing hit deals through this armor — "a vest still
/// bruises" (`weapons-and-armor.md` §"Armor stats": `floor`).
///
/// Formula step 2 clamps the resolved damage up to this: `dmg = max(floor, …)`.
/// An `i32` to share the signed arithmetic of the formula (it is compared and
/// `max`'d against the subtraction result). Private inner + derived [`Deref`]
/// (house style); a magnitude is TBD tuning.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArmorFloor(i32);

impl ArmorFloor {
    /// Build an armor floor from its px-damage magnitude (TBD tuning).
    #[must_use]
    pub const fn new(floor: i32) -> Self {
        Self(floor)
    }
}

/// Damage reduction this armor soaks, down to [`ArmorFloor`]
/// (`weapons-and-armor.md` §"Armor stats": `protection`).
///
/// Formula step 2 subtracts it from incoming damage, but penetration eats into
/// it (`damage − max(0, protection − effPen)`), and step 3 wears the armor by
/// `min(protection, damage)`. An `i32` for the signed `protection − effPen`
/// subtraction. Private inner + derived [`Deref`]; a magnitude is TBD tuning.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArmorProtection(i32);

impl ArmorProtection {
    /// Build an armor protection value from its magnitude (TBD tuning).
    #[must_use]
    pub const fn new(protection: i32) -> Self {
        Self(protection)
    }
}

/// How much this armor can block before it is useless — its durability
/// (`weapons-and-armor.md` §"Armor stats": `integrity`).
///
/// Formula step 3 degrades it per hit (`integrity −= min(protection, damage) +
/// effPen + shred`); at `integrity ≤ 0` the armor stops protecting for the rest
/// of the battle. This is the **one mutable wear field** of [`WornArmor`], so it
/// derives [`DerefMut`] (the house rule: `DerefMut` only where the inner value is
/// mutated through the newtype) and must track **below zero**, which is why the
/// inner type is a *signed* `i32`. A magnitude is TBD tuning.
#[derive(Deref, DerefMut, Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArmorIntegrity(i32);

impl ArmorIntegrity {
    /// Build an armor integrity (durability) value from its magnitude (TBD
    /// tuning).
    #[must_use]
    pub const fn new(integrity: i32) -> Self {
        Self(integrity)
    }
}

/// The amount of weapon punch this armor ignores — penetration it shrugs off
/// (`weapons-and-armor.md` §"Armor stats": `hardness`).
///
/// Formula step 1 subtracts it from punch (`effPen = max(0, punch − hardness)`).
/// Per the doc, **hardness does not degrade** — shred attacks durability, not
/// hardness — so this newtype is intentionally *immutable* through the worn copy
/// (no [`DerefMut`]). An `i32` for the signed `punch − hardness` subtraction;
/// a magnitude is TBD tuning.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArmorHardness(i32);

impl ArmorHardness {
    /// Build an armor hardness value from its magnitude (TBD tuning).
    #[must_use]
    pub const fn new(hardness: i32) -> Self {
        Self(hardness)
    }
}

/// One of the six body locations a hit can land on (`docs/combat/resolution.md`
/// §4: "Head · Torso · L-Arm · R-Arm · L-Leg · R-Leg").
///
/// Introduced here as the key for armor-by-location ([`SourceArmor`] /
/// [`WornArmor`]); the §4 weighted `roll_body_part` (a later ticket) picks over
/// the same six parts. A named domain enum, not a bare index — it is the keying
/// vocabulary for everything per-location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BodyPart {
    /// The head — rarely struck, but severity-amplifying when it is.
    Head,
    /// The torso — the bulk of the silhouette, most hits land here.
    Torso,
    /// The left arm.
    LeftArm,
    /// The right arm.
    RightArm,
    /// The left leg.
    LeftLeg,
    /// The right leg.
    RightLeg,
}

impl BodyPart {
    /// The six body parts in canonical order (`docs/combat/resolution.md` §4) —
    /// the iteration order for seeding and per-part lookups.
    pub const ALL: [Self; 6] = [
        Self::Head,
        Self::Torso,
        Self::LeftArm,
        Self::RightArm,
        Self::LeftLeg,
        Self::RightLeg,
    ];

    /// This part's index into a six-slot per-part array (`0..6`), in the same
    /// canonical order as [`ALL`](BodyPart::ALL). Used to key the per-location
    /// armor slots without a hash map.
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Head => 0,
            Self::Torso => 1,
            Self::LeftArm => 2,
            Self::RightArm => 3,
            Self::LeftLeg => 4,
            Self::RightLeg => 5,
        }
    }
}

/// The four armor stats bundled for a **single** body location.
///
/// One piece of armor protecting one [`BodyPart`]: its [`ArmorFloor`],
/// [`ArmorProtection`], [`ArmorIntegrity`], and [`ArmorHardness`]. This is the
/// shared shape used both in the read-only roster [`SourceArmor`] record and in
/// the battle-local [`WornArmor`] copy; battle wear mutates only the worn copy's
/// [`ArmorIntegrity`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArmorPiece {
    /// Minimum damage a landing hit deals through this piece.
    pub floor:      ArmorFloor,
    /// Damage this piece soaks, down to `floor`.
    pub protection: ArmorProtection,
    /// Durability — how much it can block before it is useless (the wear field).
    pub integrity:  ArmorIntegrity,
    /// Punch this piece ignores (does not degrade).
    pub hardness:   ArmorHardness,
}

impl ArmorPiece {
    /// Build an armor piece from its four stats.
    #[must_use]
    pub const fn new(
        floor: ArmorFloor,
        protection: ArmorProtection,
        integrity: ArmorIntegrity,
        hardness: ArmorHardness,
    ) -> Self {
        Self {
            floor,
            protection,
            integrity,
            hardness,
        }
    }
}

/// The **read-only** roster armor record — the persistent representation of a
/// ganger's worn armor across all six body locations (`armor_at` keyed by
/// [`BodyPart`], `weapons-and-armor.md` §"Per-hit resolution").
///
/// This is the roster source of truth: the per-location [`ArmorPiece`]s a ganger
/// carries into a battle. Per `docs/architecture.md`'s battle-local / roster
/// separation, this record is **never** mutated during a battle — it is read once
/// to seed an owned [`WornArmor`] copy ([`WornArmor::seed_from`]). It is plain
/// data (not a [`Component`]); the battle-local copy is the component the sim
/// places on ganger entities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceArmor {
    /// The six per-location armor pieces, indexed by [`BodyPart::index`].
    pieces: [ArmorPiece; 6],
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
/// armor surface during a battle (`docs/architecture.md`: "the battle-local
/// **worn armor** … seeded at construction, that mid-battle wear mutates").
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
    /// Seed a battle-local worn copy **from** a read-only roster [`SourceArmor`]
    /// record, **by value**.
    ///
    /// This is the C4 seeding function: it produces the owned battle-local copy
    /// the sim mutates during the battle. Because [`ArmorPiece`] is `Copy` and the
    /// pieces are taken by value out of the source, no reference to the roster
    /// record survives — a later [`wear_integrity`](WornArmor::wear_integrity) on
    /// this copy can never reach the source (`docs/architecture.md`: "The roster
    /// data is never touched").
    #[must_use]
    pub const fn seed_from(source: &SourceArmor) -> Self {
        Self {
            pieces: source.pieces,
        }
    }

    /// The current worn armor piece protecting `part`.
    #[must_use]
    pub const fn at(&self, part: BodyPart) -> ArmorPiece {
        self.pieces[part.index()]
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A roster source built from arbitrary, per-location-DISTINCT magnitudes —
    /// NOT shipped tuning values. Distinct per part proves the seeding copies each
    /// slot independently (not a single value smeared across all six), and using
    /// arbitrary numbers keeps the test asserting the copy/isolation *mechanism*,
    /// never a magnitude (magnitudes are TBD tuning).
    fn arbitrary_source() -> SourceArmor {
        let pieces = [
            // Head
            ArmorPiece::new(
                ArmorFloor::new(1),
                ArmorProtection::new(11),
                ArmorIntegrity::new(21),
                ArmorHardness::new(31),
            ),
            // Torso
            ArmorPiece::new(
                ArmorFloor::new(2),
                ArmorProtection::new(12),
                ArmorIntegrity::new(22),
                ArmorHardness::new(32),
            ),
            // L-Arm
            ArmorPiece::new(
                ArmorFloor::new(3),
                ArmorProtection::new(13),
                ArmorIntegrity::new(23),
                ArmorHardness::new(33),
            ),
            // R-Arm
            ArmorPiece::new(
                ArmorFloor::new(4),
                ArmorProtection::new(14),
                ArmorIntegrity::new(24),
                ArmorHardness::new(34),
            ),
            // L-Leg
            ArmorPiece::new(
                ArmorFloor::new(5),
                ArmorProtection::new(15),
                ArmorIntegrity::new(25),
                ArmorHardness::new(35),
            ),
            // R-Leg
            ArmorPiece::new(
                ArmorFloor::new(6),
                ArmorProtection::new(16),
                ArmorIntegrity::new(26),
                ArmorHardness::new(36),
            ),
        ];
        SourceArmor::new(pieces)
    }

    /// C7(a): a worn copy seeded from a source starts EQUAL to the source,
    /// field-by-field, on every body location. Walks all six parts and compares
    /// each of the four armor fields — proving `seed_from` copies the whole record
    /// faithfully (the mechanism), independent of the magnitudes used.
    #[test]
    fn seeded_copy_equals_source_field_by_field() {
        let source = arbitrary_source();
        let worn = WornArmor::seed_from(&source);

        for part in BodyPart::ALL {
            let src = source.at(part);
            let cpy = worn.at(part);
            assert_eq!(cpy.floor, src.floor, "floor mismatch at {part:?}");
            assert_eq!(
                cpy.protection, src.protection,
                "protection mismatch at {part:?}"
            );
            assert_eq!(
                cpy.integrity, src.integrity,
                "integrity mismatch at {part:?}"
            );
            assert_eq!(cpy.hardness, src.hardness, "hardness mismatch at {part:?}");
            // And the whole piece is equal.
            assert_eq!(cpy, src, "piece mismatch at {part:?}");
        }
    }

    /// C7(b): mutating `integrity` on the worn copy leaves the source record
    /// UNCHANGED — the battle-local/roster isolation guarantee. Wears the torso's
    /// integrity on the copy, then asserts the source torso integrity is still its
    /// original value, and every OTHER worn slot is untouched too.
    #[test]
    fn wearing_worn_integrity_does_not_mutate_source() {
        let source = arbitrary_source();
        let mut worn = WornArmor::seed_from(&source);

        let original = source.at(BodyPart::Torso).integrity;
        // Arbitrary wear amount — the mechanism, not a tuned magnitude.
        worn.wear_integrity(BodyPart::Torso, ArmorIntegrity::new(5));

        // The source torso integrity is unchanged — the roster never wears.
        assert_eq!(
            source.at(BodyPart::Torso).integrity,
            original,
            "roster source integrity must NOT change when the worn copy wears",
        );
        // The worn torso integrity DID change (the wear actually applied).
        assert_eq!(
            worn.at(BodyPart::Torso).integrity,
            ArmorIntegrity::new(*original - 5),
            "worn copy integrity must reflect the applied wear",
        );
        // Every other worn slot is untouched — wear is per-location.
        for part in BodyPart::ALL {
            if part == BodyPart::Torso {
                continue;
            }
            assert_eq!(
                worn.at(part),
                source.at(part),
                "non-struck worn slot {part:?} must equal the source",
            );
        }
    }

    /// Integrity may wear to AT OR BELOW zero — the "useless at `≤ 0`" gate
    /// (`weapons-and-armor.md` step 3) needs a signed inner type. Wears past the
    /// starting value and asserts the result is negative (not clamped, not
    /// underflow-panicked), confirming the `i32` choice carries the contract.
    #[test]
    fn worn_integrity_can_fall_below_zero() {
        let source = SourceArmor::uniform(ArmorPiece::new(
            ArmorFloor::new(0),
            ArmorProtection::new(0),
            ArmorIntegrity::new(3),
            ArmorHardness::new(0),
        ));
        let mut worn = WornArmor::seed_from(&source);

        worn.wear_integrity(BodyPart::Head, ArmorIntegrity::new(10));

        assert!(
            *worn.at(BodyPart::Head).integrity < 0,
            "worn integrity must be able to fall below zero (useless-at-≤0 gate)",
        );
        // The source is, again, untouched.
        assert_eq!(source.at(BodyPart::Head).integrity, ArmorIntegrity::new(3));
    }

    /// The six [`BodyPart`] variants index distinctly across `0..6` — the keying
    /// invariant the per-location slot array relies on (no two parts collide).
    #[test]
    fn body_parts_index_distinctly() {
        let mut indices: Vec<usize> = BodyPart::ALL.iter().map(|p| p.index()).collect();
        indices.sort_unstable();
        assert_eq!(indices, vec![0, 1, 2, 3, 4, 5]);
    }

    /// The newtypes' derived [`Deref`] reaches their inner `i32`, and
    /// [`ArmorIntegrity`]'s [`DerefMut`] writes through it. Built from arbitrary
    /// literals so this pins the Deref mechanism, not a magnitude.
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
}
