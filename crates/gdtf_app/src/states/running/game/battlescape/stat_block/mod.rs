mod build;
mod colors;
mod components;
mod labels;
mod portrait;
mod update;
mod writers;

#[cfg(test)]
mod test;

pub(in crate::states::running::game::battlescape) use build::spawn_stat_block;
pub(in crate::states::running::game::battlescape) use components::StatBlockRefs;
pub(crate) use update::{StatBlockData, StatBlockDataItem};
pub(in crate::states::running::game::battlescape) use update::{
    StatBlockWidgets, clear_stat_block, update_stat_block,
};

#[cfg(feature = "headless_test")]
pub(crate) mod test_support {
    pub use super::{
        components::{
            StatFaction, StatHpBar, StatHpLabel, StatInjuryLine, StatInjuryList, StatName,
            StatPortrait, StatStance, StatTuBar, StatTuLabel, StatWoundLine, StatWoundList,
            StatWoundsPips,
        },
        portrait::portrait_index_for_name,
    };
}
