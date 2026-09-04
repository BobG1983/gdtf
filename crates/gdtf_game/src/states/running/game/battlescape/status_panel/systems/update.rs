use bevy::prelude::*;
use gdtf_battle_input::SelectedShooter;

use crate::states::running::game::battlescape::{
    stat_block::{
        StatBlockData, StatBlockRefs, StatBlockWidgets, clear_stat_block, update_stat_block,
    },
    status_panel::components::StatusStatBlock,
};

pub(in crate::states::running::game::battlescape) fn update_status_panel(
    selected: Res<SelectedShooter>,
    blocks: Query<&StatBlockRefs, With<StatusStatBlock>>,
    data: Query<StatBlockData>,
    mut widgets: StatBlockWidgets,
) {
    let Ok(&refs) = blocks.single() else {
        return;
    };

    match (**selected).and_then(|entity| data.get(entity).ok()) {
        Some(ganger) => update_stat_block(refs, &ganger, &mut widgets),
        None => clear_stat_block(refs, &mut widgets),
    }
}
