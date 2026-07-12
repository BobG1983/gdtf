//! The GTW-596 cross-level tactical badges — "Signals, Not Scenery" (GTW-593
//! Option 2): compact corner badges surfacing what the active storey's terrain
//! draw cannot — a fog-VISIBLE enemy above/below (`Threat`), a hole/ledge cell's
//! drop depth (`DropDepth`), and a stair/ladder connector's level-delta
//! (`ConnectorDelta`).
//!
//! One presenter-side DERIVE system ([`derive_cross_level_signals`](crate::derive_cross_level_signals))
//! reads the sim through the SAME pure fog seams [`present_fog`](crate::present_fog) /
//! [`resolve_ganger_visibility`](crate::resolve_ganger_visibility) already use
//! (never a parallel visibility check), aggregating + capping into the
//! [`CrossLevelSignals`](crate::CrossLevelSignals) resource (the RESOLVED SPEC's
//! per-cell dedupe + 3-badge cap, `aggregate.rs`); one DRAW system
//! ([`draw_cross_level_signals`](crate::draw_cross_level_signals)) renders it,
//! change-driven off EITHER that resource OR the active storey (a badge's drawn
//! Z-band is hard-cut to [`ActiveLevel`](crate::ActiveLevel), so a level switch
//! must redraw even on the rare frame where the two storeys' derived signal sets
//! happen to be byte-identical).

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
