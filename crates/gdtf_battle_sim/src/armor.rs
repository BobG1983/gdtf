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
//!
//! ## E3.1 — the armor-type vocabulary
//!
//! The E3.1 slice adds [`ArmorType`] (the armor half of the dual vocabulary of
//! `docs/combat/matchup.md` §"The 7 types") and gives every [`ArmorPiece`] an
//! `armor_type` so a struck piece carries its wheel node. [`ArmorType`]'s seven
//! variants name the same seven wheel nodes as [`crate::weapon::DamageType`], node
//! `i` of one mirroring node `i` of the other — one tournament wheel drives both
//! sides. This is the DATA substrate only; the matchup lookup (the wheel itself)
//! is a later E3 slice, not here.

use bevy::prelude::{Component, Deref, DerefMut};
use serde::Deserialize;

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
    /// Build an armor floor from its flat-damage-reduction magnitude (TBD tuning).
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

/// The **armor type** a piece is — one of the seven shared wheel nodes
/// (`docs/combat/matchup.md` §"The 7 types", Table 1).
///
/// This is the armor half of the **dual vocabulary**: each variant names the same
/// wheel node `#` as the mirror [`crate::weapon::DamageType`] variant (node `i` of
/// one mirrors node `i` of the other), so one tournament wheel drives both sides
/// (matchup.md §"The 7 types": "same wheel node `#`, two names"). The order here
/// pins the node index — variant `i` is wheel node `i`:
///
/// ```text
/// 0 Plated · 1 Refractive · 2 Flak · 3 Void · 4 Hazard · 5 Reinforced · 6 Ceramic
/// ```
///
/// A named domain enum, not a bare `u8` (no-bare-types). The matchup lookup itself
/// — which armor node resists which weapon node — is a later E3 slice; this only
/// fixes the vocabulary and its node order. `Deserialize` so an armor piece's
/// authored data names its type by variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum ArmorType {
    /// Wheel node 0 — mirror of [`DamageType::Shock`](crate::weapon::DamageType::Shock).
    Plated,
    /// Wheel node 1 — mirror of [`DamageType::Blast`](crate::weapon::DamageType::Blast).
    Refractive,
    /// Wheel node 2 — mirror of [`DamageType::Chem`](crate::weapon::DamageType::Chem).
    Flak,
    /// Wheel node 3 — mirror of [`DamageType::Kinetic`](crate::weapon::DamageType::Kinetic).
    Void,
    /// Wheel node 4 — mirror of [`DamageType::Plasma`](crate::weapon::DamageType::Plasma).
    Hazard,
    /// Wheel node 5 — mirror of [`DamageType::Rend`](crate::weapon::DamageType::Rend).
    Reinforced,
    /// Wheel node 6 — mirror of [`DamageType::Las`](crate::weapon::DamageType::Las).
    Ceramic,
}

impl ArmorType {
    /// The seven armor types in **wheel-node order** (`docs/combat/matchup.md`
    /// Table 1) — index `i` is wheel node `i`, the mirror of
    /// [`crate::weapon::DamageType::ALL`]`[i]`. The exhaustive sweep the
    /// variant-count and dual-vocabulary-parity tests iterate.
    pub const ALL: [Self; 7] = [
        Self::Plated,
        Self::Refractive,
        Self::Flak,
        Self::Void,
        Self::Hazard,
        Self::Reinforced,
        Self::Ceramic,
    ];

    /// The default armor type when a source does not specify one
    /// (`docs/combat/matchup.md` §"The 7 types"). Node 0 — [`Plated`](ArmorType::Plated)
    /// — is the canonical first wheel node; seed sites with no authored type
    /// (e.g. cover, or a roster source predating per-piece types) fall back to it
    /// rather than carry an absent value. A neutral, explicit default, not a
    /// gameplay claim — the magnitudes and per-piece authoring are TBD.
    pub const DEFAULT: Self = Self::Plated;
}

/// The four armor stats bundled for a **single** body location, plus the piece's
/// [`ArmorType`] (its matchup-wheel node).
///
/// One piece of armor protecting one [`BodyPart`]: its [`ArmorFloor`],
/// [`ArmorProtection`], [`ArmorIntegrity`], [`ArmorHardness`], and its
/// [`ArmorType`]. This is the shared shape used both in the read-only roster
/// [`SourceArmor`] record and in the battle-local [`WornArmor`] copy; battle wear
/// mutates only the worn copy's [`ArmorIntegrity`] (the type and the other stats
/// are immutable through the worn copy).
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
    /// The piece's matchup-wheel node (its [`ArmorType`]).
    pub armor_type: ArmorType,
}

impl ArmorPiece {
    /// Build an armor piece from its four stats and its [`ArmorType`].
    #[must_use]
    pub const fn new(
        floor: ArmorFloor,
        protection: ArmorProtection,
        integrity: ArmorIntegrity,
        hardness: ArmorHardness,
        armor_type: ArmorType,
    ) -> Self {
        Self {
            floor,
            protection,
            integrity,
            hardness,
            armor_type,
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
            assert_eq!(
                cpy.armor_type, src.armor_type,
                "armor_type mismatch at {part:?}"
            );
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
            ArmorType::DEFAULT,
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

    /// [`WornArmor::protects`] tracks the "useless at `≤ 0`" gate: a piece with
    /// positive integrity protects, and once worn to `≤ 0` it stops protecting
    /// (later hits resolve as bare flesh — `weapons-and-armor.md` step 3). Wears a
    /// piece from positive, through exactly zero, to negative and asserts the
    /// predicate flips at the crossing and stays false thereafter.
    #[test]
    fn protects_is_false_at_or_below_zero() {
        let source = SourceArmor::uniform(ArmorPiece::new(
            ArmorFloor::new(0),
            ArmorProtection::new(0),
            ArmorIntegrity::new(2),
            ArmorHardness::new(0),
            ArmorType::DEFAULT,
        ));
        let mut worn = WornArmor::seed_from(&source);

        // Positive integrity ⇒ still protecting.
        assert!(
            worn.protects(BodyPart::Torso),
            "a piece with integrity > 0 must protect",
        );

        // Wear exactly TO zero — the boundary itself is unprotected (≤ 0, not < 0).
        worn.wear_integrity(BodyPart::Torso, ArmorIntegrity::new(2));
        assert_eq!(*worn.at(BodyPart::Torso).integrity, 0);
        assert!(
            !worn.protects(BodyPart::Torso),
            "a piece worn to exactly 0 must stop protecting (≤ 0 gate)",
        );

        // Wear further below zero — still unprotected.
        worn.wear_integrity(BodyPart::Torso, ArmorIntegrity::new(5));
        assert!(
            *worn.at(BodyPart::Torso).integrity < 0,
            "integrity must fall below zero",
        );
        assert!(
            !worn.protects(BodyPart::Torso),
            "a piece worn below zero must stay unprotected (bare flesh)",
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

    /// AC2 — `ArmorType` has exactly 7 variants, in wheel-node order
    /// (`docs/combat/matchup.md` Table 1).
    #[test]
    fn armor_type_has_seven_variants() {
        assert_eq!(ArmorType::ALL.len(), 7);
        assert_eq!(
            ArmorType::ALL,
            [
                ArmorType::Plated,
                ArmorType::Refractive,
                ArmorType::Flak,
                ArmorType::Void,
                ArmorType::Hazard,
                ArmorType::Reinforced,
                ArmorType::Ceramic,
            ]
        );
    }

    /// AC2 (dual-vocabulary parity) — both wheels are length 7 and node `i` of
    /// [`ArmorType::ALL`] mirrors node `i` of [`crate::weapon::DamageType::ALL`]
    /// (matchup.md §"The 7 types": "same wheel node `#`, two names"). This is the
    /// cross-enum half AC2 requires; it lives here because it imports both enums.
    #[test]
    fn dual_vocabulary_nodes_parity() {
        use crate::weapon::DamageType;

        // Same node count — neither vocabulary can drift from the shared wheel.
        assert_eq!(ArmorType::ALL.len(), DamageType::ALL.len());
        assert_eq!(ArmorType::ALL.len(), 7);

        // matchup.md Table 1: node # ↦ (Armor, Weapon/Damage). Node `i` of each
        // `ALL` array must be exactly this mirror pair.
        let expected_mirror = [
            (ArmorType::Plated, DamageType::Shock),
            (ArmorType::Refractive, DamageType::Blast),
            (ArmorType::Flak, DamageType::Chem),
            (ArmorType::Void, DamageType::Kinetic),
            (ArmorType::Hazard, DamageType::Plasma),
            (ArmorType::Reinforced, DamageType::Rend),
            (ArmorType::Ceramic, DamageType::Las),
        ];
        for (i, (armor, damage)) in expected_mirror.into_iter().enumerate() {
            assert_eq!(ArmorType::ALL[i], armor, "armor node {i} drifted");
            assert_eq!(DamageType::ALL[i], damage, "damage node {i} drifted");
        }
    }

    /// AC3 — an `ArmorPiece` carries an `ArmorType` and reads it back (mechanism,
    /// not magnitude). Pairs with `weapon_carries_damage_type_round_trip` in
    /// `weapon.rs`.
    #[test]
    fn armor_piece_carries_armor_type() {
        let piece = ArmorPiece::new(
            ArmorFloor::new(1),
            ArmorProtection::new(2),
            ArmorIntegrity::new(3),
            ArmorHardness::new(4),
            ArmorType::Ceramic,
        );
        assert_eq!(piece.armor_type, ArmorType::Ceramic);

        // And it survives the seed copy onto a worn piece (the field is part of the
        // copied record, like the four stats).
        let worn = WornArmor::seed_from(&SourceArmor::uniform(piece));
        assert_eq!(worn.at(BodyPart::Torso).armor_type, ArmorType::Ceramic);
    }
}
