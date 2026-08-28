//! Ordered log of sim acts for replay, QA, and UI.

mod deed;
mod entry;
pub mod facts;
mod log;
mod plugin;
mod provenance;
mod record;
mod seq;
mod witness;

#[cfg(test)]
mod test;

pub use deed::ActDeed;
pub use entry::{ActEntry, RecordedAct};
pub use facts::{MagazineFacts, PoseFacts, PositionFacts, SuppressedNow, VitalsFacts};
pub use log::{ActLog, SquadSees};
pub use plugin::wire_act_log;
pub use provenance::ActProvenance;
pub use record::record_acts;
pub use seq::ActSeq;
pub use witness::{ActWitnesses, WatchingFactions};
