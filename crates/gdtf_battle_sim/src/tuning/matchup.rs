//! Armor-type matchup multipliers.

use serde::Deserialize;

use crate::matchup::MatchupMultiplier;

/// Favorable / neutral / resisted multipliers.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct MatchupMultipliers {
    /// Advantageous matchup.
    pub favorable: MatchupMultiplier,
    /// Neutral matchup.
    pub neutral:   MatchupMultiplier,
    /// Resisted matchup.
    pub resisted:  MatchupMultiplier,
}

impl Default for MatchupMultipliers {
    fn default() -> Self {
        Self {
            favorable: MatchupMultiplier::new(1.33),
            neutral:   MatchupMultiplier::new(1.0),
            resisted:  MatchupMultiplier::new(0.34),
        }
    }
}
