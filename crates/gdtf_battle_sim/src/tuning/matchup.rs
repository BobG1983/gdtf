//! The 7-type matchup multipliers (E3.2; `docs/combat/matchup.md`).

use serde::Deserialize;

use crate::matchup::MatchupMultiplier;

/// The 7-type **matchup multipliers** — the punch-&-shred scalars each
/// [`crate::matchup::Matchup`] outcome applies (`docs/combat/matchup.md`
/// §"Modifier never auto-win": favorable ×1.33 / neutral ×1.0 / resisted ×0.34).
///
/// Three [`MatchupMultiplier`] tuning COEFFICIENTS, one per outcome. The
/// asymmetry is deliberate: the resisted penalty (−66%) is **double** the
/// favorable bonus (+33%), pressuring players to *avoid* bad matchups, not just
/// chase good ones. Scale-independent — a proportional swing is always felt and
/// never auto-wins (matchup.md §"Why a multiplier"). The matchup modifies punch &
/// shred ONLY (resolution.md §5); these scalars never touch any other stat.
/// Magnitudes are **tunable** — value-agnostic tests only (ordering, not value).
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct MatchupMultipliers {
    /// Favorable — the weapon type beats the armor type (doc default ×1.33).
    pub favorable: MatchupMultiplier,
    /// Neutral — same wheel node / mirror (doc default ×1.0, unchanged).
    pub neutral:   MatchupMultiplier,
    /// Resisted — the weapon type loses to the armor type (doc default ×0.34).
    pub resisted:  MatchupMultiplier,
}

impl Default for MatchupMultipliers {
    fn default() -> Self {
        // Matchup multipliers from docs/combat/matchup.md §"Modifier never
        // auto-win" (favorable ×1.33 / neutral ×1.0 / resisted ×0.34). The
        // resisted penalty is double the favorable bonus by design — tunable
        // balance data, asserted by ordering, never by magnitude.
        Self {
            favorable: MatchupMultiplier::new(1.33),
            neutral:   MatchupMultiplier::new(1.0),
            resisted:  MatchupMultiplier::new(0.34),
        }
    }
}
