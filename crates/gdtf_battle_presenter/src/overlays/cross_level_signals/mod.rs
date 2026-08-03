mod aggregate;
mod connector;
mod derive;
mod draw;
mod drop_depth;
mod threat;
mod types;

#[cfg(test)]
mod test;

pub use derive::{CrossLevelSimFacts, derive_cross_level_signals};
pub use draw::{CrossLevelBadgeLabel, CrossLevelBadgeTile, draw_cross_level_signals};
pub use types::{
    BADGE_CAP_PER_CELL, CrossLevelBadgeKind, CrossLevelSignals, LevelDelta, ThreatCount,
};
