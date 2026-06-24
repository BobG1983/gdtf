//! `Update` (in `AppState::Load`): poll the loads, then resolve or fall back.
//!
//! The orchestrator [`poll_and_resolve`] drives one poll per frame; each per-asset
//! resolve concern lives in its own sibling module: the theme (inline in the
//! orchestrator + its `fall_back` helper, in `poll`), the tuning, the situation, the
//! weapons registry, and the armor registry (GTW-269). The `params` module holds the
//! two `SystemParam` bundles the orchestrator reads.

pub(in crate::states::load) mod armor;
mod params;
mod poll;
mod situation;
pub(in crate::states::load) mod stat_tuning;
pub(in crate::states::load) mod tuning;
pub(in crate::states::load) mod weapons;

pub(in crate::states::load) use armor::redrive_armor_on_asset_event;
pub(in crate::states::load) use poll::poll_and_resolve;
pub(in crate::states::load) use stat_tuning::redrive_stat_tuning_on_asset_event;
pub(in crate::states::load) use tuning::redrive_combat_tuning_on_asset_event;
pub(in crate::states::load) use weapons::redrive_weapons_on_asset_event;

#[cfg(test)]
pub(super) mod hot_reload_test_support;
