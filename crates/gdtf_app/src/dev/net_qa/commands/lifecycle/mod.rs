//! Commands that start and end a battle.
pub(crate) mod flee;
pub(crate) mod start;

pub(in crate::dev::net_qa) use flee::BattleFlee;
pub(in crate::dev::net_qa) use start::BattleStart;
