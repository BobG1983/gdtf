//! The CONSEQUENCE family of recorders (GTW-727 C9, family 5 of 6) — everything an act
//! LEFT BEHIND.
//!
//! Split by SOURCE, which is the family's real change-reason boundary: `messages` records
//! the consequences a sim signal already announces (reload / injury / fall / melee / death
//! / suppression / armor break / the three affliction starts), while `state` records the
//! settled after-values no message owns (vitals, magazine) as transitions against the
//! log's prior-value maps. Adding a new consequence SIGNAL touches only `messages`; adding
//! a new mirrored STATE touches only `state`.
//!
//! Wiring only.

mod messages;
mod state;

pub(super) use messages::record_consequence_messages;
pub(super) use state::{record_magazines, record_vitals};
