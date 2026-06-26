//! The per-location armor STATS: the four armor-stat newtypes, the [`BodyPart`]
//! key, the [`ArmorType`] wheel node, and the [`ArmorPiece`] that bundles them.

use bevy::prelude::{Component, Deref, DerefMut};
use serde::Deserialize;

/// The minimum damage a landing hit deals through this armor — "a vest still
/// bruises" (`weapons-and-armor.md` §"Armor stats": `floor`).
///
/// Formula step 2 clamps the resolved damage up to this: `dmg = max(floor, …)`.
/// An `i32` to share the signed arithmetic of the formula (it is compared and
/// `max`'d against the subtraction result). Private inner + derived [`Deref`]
/// (house style); a magnitude is TBD tuning. `#[serde(transparent)]` lets an
/// authored floor parse as a bare integer.
/// `Default` (`ArmorFloor(0)`) is a **spawn-seed sentinel only** — the `bsn!`
/// spawn path seeds the [`ArmorPiece`] / armor-piece entity (related via
/// [`Wears`](super::Wears)) component slot via `Default` before the authored value
/// overwrites it (GTW-322). It is NOT a valid authored armor stat.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Default)]
#[serde(transparent)]
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
/// `#[serde(transparent)]` lets an authored protection parse as a bare integer.
/// `Default` (`ArmorProtection(0)`) is a **spawn-seed sentinel only** — seeded by
/// the `bsn!` spawn path before the authored value overwrites it (GTW-322). NOT a
/// valid authored armor stat.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Default)]
#[serde(transparent)]
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
/// of the battle. This is the **one mutable wear field** of an armor-piece entity
/// ([`wear_armor`](crate::armor_wear::wear_armor) mutates this in place), so it derives
/// [`DerefMut`] (the house rule: `DerefMut` only where the inner value is mutated
/// through the newtype) and must track **below zero**, which is why the inner type
/// is a *signed* `i32`. A magnitude is TBD tuning.
/// `#[serde(transparent)]` lets an authored integrity parse as a bare integer.
/// `Default` (`ArmorIntegrity(0)`) is a **spawn-seed sentinel only** — seeded by
/// the `bsn!` spawn path before the authored value overwrites it (GTW-322). NOT a
/// valid authored armor stat (`0` would read as already-useless armor).
#[derive(
    Deref, DerefMut, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Default,
)]
#[serde(transparent)]
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
/// a magnitude is TBD tuning. `#[serde(transparent)]` lets an authored hardness
/// parse as a bare integer.
/// `Default` (`ArmorHardness(0)`) is a **spawn-seed sentinel only** — seeded by
/// the `bsn!` spawn path before the authored value overwrites it (GTW-322). NOT a
/// valid authored armor stat.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Default)]
#[serde(transparent)]
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
/// Introduced here as the key for armor-by-location ([`SourceArmor`](super::worn::SourceArmor)
/// and the armor-piece entities related via [`Wears`](super::Wears), each tagged with its
/// `BodyPart`); the §4 weighted `roll_body_part` (a later ticket) picks over the same six
/// parts. A named domain enum, not a bare index — it is the keying vocabulary for
/// everything per-location.
///
/// A Bevy [`Component`] since GTW-323 (ADR-0004): it tags each worn-armor-piece
/// entity with the body location it protects, so the `struck_piece` lookup keys
/// `ganger → Wears → the BodyPart-tagged piece`. `Default` ([`BodyPart::Head`], the
/// first canonical part) is the GTW-322 `bsn!` spawn-seed sentinel — the
/// `template_value` spawn path seeds the slot via `Default` before the authored
/// per-piece tag overwrites it; it is never a meaningful default location.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
pub enum BodyPart {
    /// The head — rarely struck, but severity-amplifying when it is.
    #[default]
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
/// `Default` is [`ArmorType::Plated`] — matching the documented [`ArmorType::DEFAULT`]
/// (node 0, the canonical first wheel node). Used as the `bsn!` spawn-seed sentinel
/// (GTW-322), consistent with the existing `DEFAULT` fallback.
///
/// A Bevy [`Component`] since GTW-323 (ADR-0004): the per-piece wheel node lives on
/// the worn-armor-piece entity (the `struck_piece` matchup reads it off the piece),
/// not packed inside a ganger-side array.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Default)]
pub enum ArmorType {
    /// Wheel node 0 — mirror of [`DamageType::Shock`](crate::weapon::DamageType::Shock).
    #[default]
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
/// [`ArmorType`]. This is the shared shape used in the read-only roster
/// [`SourceArmor`](super::worn::SourceArmor) record AND as the per-piece stat
/// components spawned onto the battle-local armor-piece entities (related via
/// [`Wears`](super::Wears), ADR-0004); battle wear mutates only a piece entity's
/// [`ArmorIntegrity`] component (the type and the other stats are immutable). Derives
/// [`Deserialize`] so an authored situation's roster armor names each piece by its
/// five typed stats.
/// `Default` (all-zero stats, [`ArmorType::Plated`]) is a **spawn-seed sentinel
/// only** — seeded by the `bsn!` spawn path before the authored piece overwrites
/// it (GTW-322). NOT a valid authored piece (zero integrity reads as already-useless).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Default)]
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
