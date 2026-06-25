//! The matchup-wheel implementation — [`WheelNode`], the [`Matchup`] outcome enum, the
//! `DamageType`/`ArmorType` → node bridges, [`matchup`], and [`matchup_multiplier`]. See
//! the module docs (`super`) for the tournament construction.

use bevy::prelude::Deref;

use crate::{armor::ArmorType, tuning::CombatTuning, weapon::DamageType};

/// The number of wheel nodes in the 7-type two-paradox tournament
/// (`docs/combat/two-paradox-tournament.md` §2: the smallest 2-paradox tournament
/// has size 7). The modulus of the wheel math.
pub(super) const WHEEL_NODE_COUNT: u8 = 7;

/// A node `0..=6` on the shared 7-type matchup wheel
/// (`docs/combat/matchup.md` §"The 7 types", Table 1).
///
/// One named domain value, not a bare `u8` (no-bare-types): both
/// [`DamageType`] and [`ArmorType`] map onto the *same* wheel node, so the
/// tournament math lives here once. The inner index is private and derived
/// [`Deref`] reaches it; build one via [`WheelNode::new`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WheelNode(u8);

impl WheelNode {
    /// Build a wheel node from its index, reduced into `0..=6` (the wheel has
    /// [`WHEEL_NODE_COUNT`] nodes). Reducing here keeps the tournament arithmetic
    /// (`+3`/`+5`/`+6`) total without a separate clamp at each call site.
    #[must_use]
    pub const fn new(index: u8) -> Self {
        Self(index % WHEEL_NODE_COUNT)
    }

    /// The three armor nodes this (weapon) node is **strong** against — the
    /// quadratic non-residues `{(n+3)%7, (n+5)%7, (n+6)%7}`
    /// (`docs/combat/matchup.md` §"Drop-in data" / §"The math we get for free").
    ///
    /// The shipped converse orientation: weapon node `n` penetrates these three
    /// armor nodes (favorable), is resisted by the other three, and is neutral
    /// against its own mirror `n`.
    #[must_use]
    pub const fn strong_against(self) -> [Self; 3] {
        [
            Self::new(self.0 + 3),
            Self::new(self.0 + 5),
            Self::new(self.0 + 6),
        ]
    }
}

/// The result of a weapon×armor matchup lookup
/// (`docs/combat/matchup.md` §"Modifier never auto-win").
///
/// The three outcomes the wheel resolves to. A named domain enum, not a bare
/// flag: each variant selects the punch-&-shred multiplier in
/// [`matchup_multiplier`]. The matchup is a *modifier*, never an auto-win — even
/// [`Resisted`](Matchup::Resisted) still penetrates and
/// [`Favorable`](Matchup::Favorable) never bypasses armor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Matchup {
    /// The weapon type beats the armor type — punch & shred are amplified
    /// (the favorable multiplier).
    Favorable,
    /// Same wheel node (the mirror) — punch & shred are unchanged
    /// (the neutral multiplier).
    Neutral,
    /// The weapon type loses to the armor type — punch & shred are reduced
    /// (the resisted multiplier).
    Resisted,
}

impl Matchup {
    /// The three matchup outcomes, in favorable→neutral→resisted order — the
    /// exhaustive sweep helper for tests and callers iterating the outcomes.
    pub const ALL: [Self; 3] = [Self::Favorable, Self::Neutral, Self::Resisted];
}

impl DamageType {
    /// This damage type's [`WheelNode`] — its position in [`DamageType::ALL`]
    /// (`docs/combat/matchup.md` Table 1, wheel-node order). Variant `i` is wheel
    /// node `i`, the mirror of [`ArmorType::ALL`]`[i]`.
    ///
    /// The matchup wheel reasons in [`WheelNode`]s; this is the bridge from the
    /// weapon vocabulary onto the shared wheel. Defined on the ENUM (not on
    /// [`crate::weapon::Weapon`]).
    #[must_use]
    pub fn node(self) -> WheelNode {
        WheelNode::new(node_index(&Self::ALL, self))
    }
}

impl ArmorType {
    /// This armor type's [`WheelNode`] — its position in [`ArmorType::ALL`]
    /// (`docs/combat/matchup.md` Table 1, wheel-node order). Variant `i` is wheel
    /// node `i`, the mirror of [`DamageType::ALL`]`[i]`.
    ///
    /// The matchup wheel reasons in [`WheelNode`]s; this is the bridge from the
    /// armor vocabulary onto the shared wheel. Defined on the ENUM (not on
    /// [`crate::armor::ArmorPiece`]).
    #[must_use]
    pub fn node(self) -> WheelNode {
        WheelNode::new(node_index(&Self::ALL, self))
    }
}

/// The index of `needle` within `all` (an `ALL` array in wheel-node order),
/// as a `u8` wheel index — the shared `node()` body for both vocabularies.
///
/// Both `ALL` arrays are length 7 and ordered by wheel node (matchup.md Table 1),
/// so a variant's position *is* its wheel node. A missing variant cannot occur
/// (the search is over the exhaustive `ALL`); it falls back to node 0 rather than
/// panic (no `unwrap`/`expect` in the sim).
fn node_index<T: PartialEq>(all: &[T; WHEEL_NODE_COUNT as usize], needle: T) -> u8 {
    for (index, candidate) in all.iter().enumerate() {
        if *candidate == needle {
            // `index < 7` (the array length), so it always fits a `u8`; the
            // fallback `0` can never be taken in practice.
            return u8::try_from(index).unwrap_or(0);
        }
    }
    0
}

/// Resolve the matchup of a weapon's [`DamageType`] against an armor's
/// [`ArmorType`] via the shared tournament wheel
/// (`docs/combat/matchup.md` §"Drop-in data").
///
/// Same wheel node → [`Matchup::Neutral`]; the armor node in the weapon node's
/// [`WheelNode::strong_against`] set → [`Matchup::Favorable`]; otherwise
/// [`Matchup::Resisted`]. One lookup over the 7-type wheel — never a 49-cell
/// chart.
#[must_use]
pub fn matchup(weapon: DamageType, armor: ArmorType) -> Matchup {
    let weapon_node = weapon.node();
    let armor_node = armor.node();
    if weapon_node == armor_node {
        Matchup::Neutral
    } else if weapon_node.strong_against().contains(&armor_node) {
        Matchup::Favorable
    } else {
        Matchup::Resisted
    }
}

/// A **matchup multiplier** — the scalar a [`Matchup`] applies to weapon `punch`
/// and `shred` (`docs/combat/matchup.md` §"Modifier never auto-win": favorable
/// ×1.33 / neutral ×1.0 / resisted ×0.34).
///
/// A tuning COEFFICIENT, not a bare `f32` (no-bare-types). One newtype shared by
/// the three fields of [`crate::tuning::MatchupMultipliers`]: each is the same
/// *kind* of value, a punch-&-shred multiplier. Private inner + derived [`Deref`];
/// `#[serde(transparent)]` so it parses a bare RON scalar. The magnitudes are
/// **tunable** — tests assert the resisted < neutral < favorable *ordering*, never
/// a magnitude.
#[derive(Deref, Debug, Clone, Copy, PartialEq, serde::Deserialize)]
#[serde(transparent)]
pub struct MatchupMultiplier(f32);

impl MatchupMultiplier {
    /// Build a matchup multiplier from its magnitude (a dimensionless punch-&-shred
    /// scale; TBD tuning).
    #[must_use]
    pub const fn new(multiplier: f32) -> Self {
        Self(multiplier)
    }
}

/// The punch-&-shred [`MatchupMultiplier`] a [`Matchup`] yields, read from the
/// tuning store (`docs/combat/matchup.md` §"Modifier never auto-win",
/// resolution.md §5).
///
/// Takes ONLY the [`Matchup`] and the [`CombatTuning`] and returns ONLY a
/// [`MatchupMultiplier`] — by construction, no `damage` / `floor` / `protection` /
/// `integrity` / `hardness` is reachable from this signature, so the modifier can
/// touch nothing but punch & shred (the actual application is a later E3 slice).
#[must_use]
pub const fn matchup_multiplier(matchup: Matchup, tuning: &CombatTuning) -> MatchupMultiplier {
    let multipliers = tuning.matchup_multipliers;
    match matchup {
        Matchup::Favorable => multipliers.favorable,
        Matchup::Neutral => multipliers.neutral,
        Matchup::Resisted => multipliers.resisted,
    }
}
