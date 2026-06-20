//! Repaints the status panel's shared stat block from the selected player ganger
//! (GTW-278).
//!
//! [`update_status_panel`] reads [`Res<SelectedShooter>`](gdtf_battle_input::SelectedShooter),
//! resolves it to the selected [`Entity`], reads the ganger's
//! [`StatBlockData`](super::super::super::stat_block::StatBlockData), and drives the
//! shared stat-block updater
//! ([`update_stat_block`](super::super::super::stat_block::update_stat_block)). With no
//! selection — or a selected entity missing the stat-block components — the block shows the
//! empty "No ganger selected" state ([`clear_stat_block`](super::super::super::stat_block::clear_stat_block)).
//! Never stale data, never a panic (AC4).
//!
//! It runs in `Update` gated `run_if(resource_exists::<BattleInProgress>)` (the live-battle
//! witness, `bevy-traps.md` #1), `.after(InputSystems::Gather)` (GTW-264 — so it observes
//! the same update's auto-select write to `SelectedShooter`).

use bevy::prelude::*;
use gdtf_battle_input::SelectedShooter;

use crate::states::running::game::battlescape::{
    stat_block::{
        StatBlockData, StatBlockRefs, StatBlockWidgets, clear_stat_block, update_stat_block,
    },
    status_panel::components::StatusStatBlock,
};

/// Repaints the status panel's stat block from the current [`SelectedShooter`].
///
/// Looks up THIS panel's [`StatBlockRefs`] (the one marked [`StatusStatBlock`]), resolves
/// the selection to its [`StatBlockData`] (ONE `.get` over the wide read-only query), and
/// either drives the shared
/// [`update_stat_block`](super::super::super::stat_block::update_stat_block) (selected) or
/// the shared [`clear_stat_block`](super::super::super::stat_block::clear_stat_block) empty
/// state (no selection / missing components) — AC4: no panic, no stale data.
///
/// Param-only (`bevy-traps.md` #7): [`Res<SelectedShooter>`] + the read-only [`StatBlockData`]
/// query + the panel's [`StatBlockRefs`] lookup + the [`StatBlockWidgets`] write bundle.
pub(in crate::states::running::game::battlescape) fn update_status_panel(
    selected: Res<SelectedShooter>,
    blocks: Query<&StatBlockRefs, With<StatusStatBlock>>,
    data: Query<StatBlockData>,
    mut widgets: StatBlockWidgets,
) {
    // This panel's stat-block handle (exactly one in a live battle).
    let Ok(&refs) = blocks.single() else {
        return;
    };

    match (**selected).and_then(|entity| data.get(entity).ok()) {
        Some(ganger) => update_stat_block(refs, &ganger, &mut widgets),
        None => clear_stat_block(refs, &mut widgets),
    }
}
