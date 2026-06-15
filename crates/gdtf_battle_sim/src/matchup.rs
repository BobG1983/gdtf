//! The 7-type weapon×armor matchup lookup — the Paley-tournament wheel and the
//! punch-&-shred multiplier it yields.
//!
//! This is the E3.2 slice (`docs/combat/matchup.md`, `docs/combat/resolution.md`
//! §5, `docs/combat/two-paradox-tournament.md`). It is **pure logic over the E3.1
//! enums** ([`crate::weapon::DamageType`] / [`crate::armor::ArmorType`]) plus the
//! tuning scalars — it touches neither [`crate::weapon::Weapon`] nor
//! [`crate::armor::ArmorPiece`], and carries no pixel.
//!
//! ## The wheel
//!
//! The seven types are a **2-paradox tournament** (a directed Paley graph, n=2,
//! OEIS A362137; `docs/combat/two-paradox-tournament.md`). Each type maps to a
//! [`WheelNode`] `0..=6`. The shipped game orientation
//! (`docs/combat/matchup.md` §"Drop-in data") is the quadratic **non-residues**:
//! weapon node `w` penetrates armor nodes `{(w+3)%7, (w+5)%7, (w+6)%7}`. So a
//! matchup resolves in one lookup:
//!
//! - same node (the mirror) → [`Matchup::Neutral`];
//! - armor node in the weapon node's strong set → [`Matchup::Favorable`];
//! - else → [`Matchup::Resisted`].
//!
//! Each type is favorable vs exactly 3, resisted vs exactly 3, neutral vs 1 (its
//! own mirror) — a balanced regular tournament, balanced *by construction* rather
//! than by hand-authoring 49 cells.
//!
//! ## Modifier, never auto-win
//!
//! The matchup is a **multiplier on weapon `punch` and `shred` only**
//! (`docs/combat/matchup.md` §"Modifier never auto-win", resolution.md §5) — it
//! does NOT touch `damage`, `floor`, `protection`, `integrity`, or `hardness`.
//! [`matchup_multiplier`] yields that scalar from [`crate::tuning::CombatTuning`]
//! (favorable ×1.33 / neutral ×1.0 / resisted ×0.34 are TUNING defaults). Its
//! signature can see *only* a [`Matchup`] and the tuning, so no other stat is
//! reachable from it — the actual application to punch & shred is a later E3 slice.

use bevy::prelude::Deref;

use crate::{armor::ArmorType, tuning::CombatTuning, weapon::DamageType};

/// The number of wheel nodes in the 7-type two-paradox tournament
/// (`docs/combat/two-paradox-tournament.md` §2: the smallest 2-paradox tournament
/// has size 7). The modulus of the wheel math.
const WHEEL_NODE_COUNT: u8 = 7;

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

#[cfg(test)]
mod tests {
    use super::*;

    /// AC1 — sweep all 49 (`DamageType` × `ArmorType`) pairs and assert that
    /// EXACTLY the 7 mirror-node pairs (node `i` weapon vs node `i` armor) are
    /// [`Matchup::Neutral`], and no others.
    #[test]
    fn neutral_iff_same_wheel_node() {
        let mut neutral_count = 0u32;
        for weapon in DamageType::ALL {
            for armor in ArmorType::ALL {
                let same_node = *weapon.node() == *armor.node();
                let result = matchup(weapon, armor);
                if same_node {
                    assert_eq!(
                        result,
                        Matchup::Neutral,
                        "mirror pair {weapon:?} vs {armor:?} must be Neutral",
                    );
                    neutral_count += 1;
                } else {
                    assert_ne!(
                        result,
                        Matchup::Neutral,
                        "non-mirror pair {weapon:?} vs {armor:?} must NOT be Neutral",
                    );
                }
            }
        }
        // Exactly the 7 mirror pairs are Neutral.
        assert_eq!(neutral_count, 7, "exactly 7 mirror pairs must be Neutral");
    }

    /// AC2 — balanced regular tournament: for each of the 7 `DamageType`s, count
    /// Favorable across the 7 `ArmorType`s == 3 and Resisted == 3 (and Neutral == 1,
    /// its own mirror).
    #[test]
    fn each_type_favorable_three_resisted_three() {
        for weapon in DamageType::ALL {
            let mut favorable = 0u32;
            let mut neutral = 0u32;
            let mut resisted = 0u32;
            for armor in ArmorType::ALL {
                match matchup(weapon, armor) {
                    Matchup::Favorable => favorable += 1,
                    Matchup::Neutral => neutral += 1,
                    Matchup::Resisted => resisted += 1,
                }
            }
            assert_eq!(favorable, 3, "{weapon:?} must be Favorable vs exactly 3");
            assert_eq!(resisted, 3, "{weapon:?} must be Resisted vs exactly 3");
            assert_eq!(
                neutral, 1,
                "{weapon:?} must be Neutral vs exactly 1 (mirror)"
            );
        }
    }

    /// AC3 — tournament antisymmetry: whenever node X (weapon) is Favorable against
    /// node Y (armor), the mirror direction — the `DamageType` at Y's node vs the
    /// `ArmorType` at X's node — is Resisted (one self-converse wheel drives both
    /// sides). Crosses between the two vocabularies via the node index.
    #[test]
    fn favorable_implies_mirror_resisted() {
        for weapon in DamageType::ALL {
            for armor in ArmorType::ALL {
                if matchup(weapon, armor) != Matchup::Favorable {
                    continue;
                }
                // Cross vocabularies at the same wheel nodes: the weapon now sits
                // at the armor's node, the armor at the weapon's node.
                let mirror_weapon = DamageType::ALL[*armor.node() as usize];
                let mirror_armor = ArmorType::ALL[*weapon.node() as usize];
                assert_eq!(
                    matchup(mirror_weapon, mirror_armor),
                    Matchup::Resisted,
                    "if {weapon:?} is Favorable vs {armor:?}, the mirror direction \
                     ({mirror_weapon:?} vs {mirror_armor:?}) must be Resisted",
                );
            }
        }
    }

    /// AC5 — load the default tuning multipliers and assert
    /// `resisted < neutral < favorable` as an ORDERING (never a magnitude). The
    /// resisted penalty is the heavier swing, the favorable bonus the lighter —
    /// the asymmetry of `docs/combat/matchup.md` §"Modifier never auto-win".
    #[test]
    fn multiplier_ordering_resisted_lt_neutral_lt_favorable() {
        let tuning = CombatTuning::default();
        let resisted = *matchup_multiplier(Matchup::Resisted, &tuning);
        let neutral = *matchup_multiplier(Matchup::Neutral, &tuning);
        let favorable = *matchup_multiplier(Matchup::Favorable, &tuning);

        assert!(
            resisted < neutral,
            "resisted multiplier must be < neutral (got {resisted} vs {neutral})",
        );
        assert!(
            neutral < favorable,
            "neutral multiplier must be < favorable (got {neutral} vs {favorable})",
        );
    }

    /// The `node()` accessor agrees with `ALL` ordering for both vocabularies, and
    /// the two are mirrors (node `i` of one matches node `i` of the other). Pins
    /// the bridge from the dual vocabulary onto the shared wheel.
    #[test]
    fn node_accessor_agrees_with_all_ordering() {
        for (i, weapon) in DamageType::ALL.into_iter().enumerate() {
            assert_eq!(
                *weapon.node(),
                i as u8,
                "{weapon:?} node must be its ALL index"
            );
        }
        for (i, armor) in ArmorType::ALL.into_iter().enumerate() {
            assert_eq!(
                *armor.node(),
                i as u8,
                "{armor:?} node must be its ALL index"
            );
        }
        // Mirror parity across the two vocabularies (matchup.md Table 1).
        for i in 0..7usize {
            assert_eq!(*DamageType::ALL[i].node(), *ArmorType::ALL[i].node());
        }
    }

    /// `WheelNode::strong_against` returns the quadratic non-residues
    /// `{(n+3)%7, (n+5)%7, (n+6)%7}` for every node, all reduced into `0..=6` and
    /// none equal to the node itself (a node is never strong against its mirror).
    #[test]
    fn strong_against_is_the_non_residue_offsets() {
        for n in 0..WHEEL_NODE_COUNT {
            let node = WheelNode::new(n);
            let strong = node.strong_against();
            let expected = [(n + 3) % 7, (n + 5) % 7, (n + 6) % 7];
            for (got, want) in strong.into_iter().zip(expected) {
                assert_eq!(*got, want, "node {n} strong-against offset drifted");
            }
            assert!(
                !strong.contains(&node),
                "node {n} must not be strong against its own mirror",
            );
        }
    }
}
