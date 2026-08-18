mod cell;
mod facing;
mod key;
mod last_save;
mod mode;
mod outcome;
mod phase;
mod refusal;
mod save_fault;
mod support;
mod terrain_kind;

pub(in crate::net_qa::wire::test) use support::{assert_ron_round_trip, assert_schema_is_usable};
