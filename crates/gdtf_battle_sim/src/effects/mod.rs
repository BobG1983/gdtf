//! Ongoing effects: bleed, DOT, fields, injuries, on-death, attachment effects.

/// Weapon attachment effect application.
pub mod attachments;
/// Bleed-out after going down.
pub mod bleed;
/// Damage-over-time effects.
pub mod dot;
/// Area field effects on cells.
pub mod fields;
/// Injury effect application helpers.
pub mod injuries;
/// Effects that fire when a ganger or cover dies.
pub mod on_death;
