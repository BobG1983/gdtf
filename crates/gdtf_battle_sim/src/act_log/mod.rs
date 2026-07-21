//! The **act log** (GTW-727) — the sim's ordered, sim-owned record of everything that
//! happened, and the mechanism that lets a view show acts one at a time without ever
//! gating the sim.
//!
//! ## Why it exists
//!
//! Before this module the sim's only account of an act was the set of world mutations it
//! left behind plus a scatter of output messages, and every view reacted to those the frame
//! they landed. A single player step that provoked two reaction-fire interrupts therefore
//! resolved both shots, both damage applications, both injuries and every log line inside
//! one frame — while the tracers were still animating. The outcome was rendered before its
//! cause: the HP had already dropped and the injury was already listed while the bolt that
//! caused them was mid-flight.
//!
//! The cure is an ordered log the sim WRITES and never waits on. The sim runs at full
//! speed; a consumer holds its own cursor and consumes at whatever pace reads well. The
//! one-way model → view dependency is untouched — nothing in this module names a presenter
//! type, and there is no back-pressure of any kind (see [`record_acts`]).
//!
//! ## Module map
//!
//! - `seq` — the counting newtypes: [`ActSeq`], [`ActLogCapacity`], [`ActLogDropped`].
//! - `provenance` — [`ActProvenance`], WHY an act happened (commanded / AI turn / reaction
//!   interrupt / engine clock).
//! - `deed` — [`ActDeed`], WHAT happened: the closed vocabulary, exhaustive over the
//!   combat log's sources plus the life-state transition and the three drawn-state
//!   snapshots.
//! - `facts` — the multi-field AFTER-value snapshots the query-sourced recorders carry
//!   ([`PoseFacts`] / [`VitalsFacts`] / [`MagazineFacts`] / [`PositionFacts`]).
//! - `entry` — [`ActEntry`] (a sequenced record) and [`RecordedAct`] (the un-sequenced
//!   triple a recorder hands to the log).
//! - `log` — [`ActLog`], the battle-lifetime ring plus the prior-value maps transitions
//!   are detected against.
//! - `record` — [`record_acts`], the ONE registered writer, and its six per-family
//!   recorders.
//! - `plugin` — [`wire_act_log`], the registration.
//!
//! Wiring only.

mod deed;
mod entry;
pub mod facts;
mod log;
mod plugin;
mod provenance;
mod record;
mod seq;

#[cfg(test)]
mod test;

pub use deed::ActDeed;
pub use entry::{ActEntry, RecordedAct};
pub use facts::{MagazineFacts, PoseFacts, PositionFacts, SuppressedNow, VitalsFacts};
pub use log::ActLog;
pub use plugin::wire_act_log;
pub use provenance::ActProvenance;
pub use record::record_acts;
pub use seq::{ActLogCapacity, ActLogDropped, ActSeq};
