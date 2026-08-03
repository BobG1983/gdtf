//! Damage-type vs armor-type matchup wheel.

#[cfg(test)]
mod test;
mod wheel;

pub use wheel::{Matchup, MatchupMultiplier, WheelNode, matchup, matchup_multiplier};
