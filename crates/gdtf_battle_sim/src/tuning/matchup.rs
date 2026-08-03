use serde::Deserialize;

use crate::matchup::MatchupMultiplier;

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct MatchupMultipliers {
        pub favorable: MatchupMultiplier,
        pub neutral:   MatchupMultiplier,
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
