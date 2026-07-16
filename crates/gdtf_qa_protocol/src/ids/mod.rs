//! The id / handle newtype vocabulary — the primitive wire identifiers every other
//! module keys off (GTW-734).
//!
//! Three concerns, one file each: entity [`token`]s (the `Entity::to_bits` carriers),
//! the coordinate wire types ([`cell`]), and the miscellaneous handles ([`handle`] —
//! fire-mode index, screenshot name, event cap, situation ref, seed, request id).
//! Every type is a private-inner newtype with a derived `Deref` and a `new`
//! constructor (no-bare-types rules 1–5), serde-transparent so it rides the wire as
//! its bare inner.

pub mod cell;
pub mod handle;
pub mod token;

pub use cell::{CellLevelNet, CellNet, CellXNet, CellYNet, LevelNet};
pub use handle::{EventCap, FireModeIndex, RequestId, SeedNet, ShotName, SituationRef};
pub use token::{DoorToken, EmplacementToken, GangerToken};

#[cfg(test)]
mod test;
